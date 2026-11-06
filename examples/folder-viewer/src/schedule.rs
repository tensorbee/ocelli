//! Two long-lived decode workers. Every decode the process server runs is a
//! job here, including a preview that carries the file's bytes, and every
//! pooled request body is allocated and freed on these two threads. The
//! connection thread reads the body into the buffer a worker allocated.
//!
//! `P` in the resident formula is 2, so there are two threads. The `4` in
//! `4 * S_max` is the most request bodies those threads keep at once. The
//! count does not follow the host core count. A pooled body is freed by the
//! worker that allocated it. A decoded frame's sample buffer that leaves the
//! cache is parked on the free list only when cached bytes plus free bytes
//! fit the budget, and is freed otherwise.

use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use super::{GreyClaim, GreyGate, ReturnedGrey, Workspace, bump_peak, lock_mutex, workspace_bytes};

/// Workers. This is `P` in the resident formula.
const WORKERS: usize = 2;
/// Request bodies in flight. This is the coefficient of `S_max`.
const BODY_SLOTS: usize = 4;
const WAIT: Duration = Duration::from_millis(50);
/// How long a decode waits for its home worker before any worker takes the
/// job. Long enough to keep the home heap when the home worker is free, and
/// short enough that a busy home worker does not hold a preview while the
/// other worker is idle.
const HOME_WAIT: Duration = Duration::from_millis(5);

thread_local! {
    static WORKSPACE: RefCell<Workspace> = RefCell::new(Workspace::default());
    static DETACHED: Cell<usize> = const { Cell::new(0) };
}

enum Job {
    /// Each call site has its own closure type. The queue stores one object.
    Run(Box<dyn FnOnce() + Send>),
    Stop,
}

struct Inner {
    txs: Vec<Sender<Job>>,
    retire: Vec<Sender<Vec<u8>>>,
    body_free: Mutex<VecDeque<usize>>,
    body_ready: Condvar,
    decode_free: Mutex<VecDeque<usize>>,
    decode_ready: Condvar,
    shut: AtomicBool,
    body_now: AtomicUsize,
    body_peak: AtomicUsize,
    accounted: Vec<AtomicUsize>,
    threads: Mutex<HashSet<ThreadId>>,
    /// The grey cap. Each worker's returned grey buffer counts inside it.
    gate: Arc<GreyGate>,
}

/// The two workers and the threads they own.
pub(super) struct Scheduler {
    inner: Arc<Inner>,
    /// `JoinHandle` is not `Sync`. The mutex lets a test share `&Scheduler`.
    joins: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

/// One live request body. Drop returns the slot.
pub(crate) struct BodyPermit {
    inner: Arc<Inner>,
    slot: usize,
    worker: usize,
}

/// One decode on one worker. Drop returns the worker.
pub(crate) struct DecodePermit {
    inner: Arc<Inner>,
    worker: usize,
}

impl Scheduler {
    pub(super) fn start(gate: Arc<GreyGate>) -> Self {
        let mut txs = Vec::with_capacity(WORKERS);
        let mut rxs = Vec::with_capacity(WORKERS);
        let mut retire_txs = Vec::with_capacity(WORKERS);
        let mut retire_rxs = Vec::with_capacity(WORKERS);
        for _ in 0..WORKERS {
            let (tx, rx) = mpsc::channel();
            txs.push(tx);
            rxs.push(rx);
            let (retire_tx, retire_rx) = mpsc::channel();
            retire_txs.push(retire_tx);
            retire_rxs.push(retire_rx);
        }
        let inner = Arc::new(Inner {
            txs,
            retire: retire_txs,
            body_free: Mutex::new(VecDeque::from_iter(0..BODY_SLOTS)),
            body_ready: Condvar::new(),
            decode_free: Mutex::new(VecDeque::from_iter(0..WORKERS)),
            decode_ready: Condvar::new(),
            shut: AtomicBool::new(false),
            body_now: AtomicUsize::new(0),
            body_peak: AtomicUsize::new(0),
            accounted: (0..WORKERS).map(|_| AtomicUsize::new(0)).collect(),
            threads: Mutex::new(HashSet::new()),
            gate,
        });
        let mut joins = Vec::with_capacity(WORKERS);
        // `start` returns once both workers are registered, so a census read
        // straight after it counts two threads.
        let started = Arc::new(std::sync::Barrier::new(WORKERS + 1));
        for (index, (rx, retire_rx)) in rxs.into_iter().zip(retire_rxs).enumerate() {
            let inner = Arc::clone(&inner);
            let started = Arc::clone(&started);
            joins.push(std::thread::spawn(move || {
                worker_loop(index, rx, retire_rx, &inner, &started);
            }));
        }
        started.wait();
        Self {
            inner,
            joins: Mutex::new(joins),
        }
    }

    pub(super) fn workspace_bytes(&self) -> usize {
        self.inner.accounted.iter().fold(0_usize, |sum, slot| {
            sum.saturating_add(slot.load(Ordering::Relaxed))
        })
    }

    /// Threads that allocate bodies, the peak overlapping bodies, and how many are live.
    pub(super) fn body_schedule(&self) -> (usize, usize, usize) {
        let threads = lock_mutex(&self.inner.threads).len();
        (
            threads,
            self.inner.body_peak.load(Ordering::Relaxed),
            self.inner.body_now.load(Ordering::Relaxed),
        )
    }

    pub(super) fn acquire_body(&self, live: &mut dyn FnMut() -> bool) -> Option<BodyPermit> {
        let slot = wait_slot(
            &self.inner.body_free,
            &self.inner.body_ready,
            &self.inner.shut,
            live,
            None,
            |_| true,
        )?;
        let now = self
            .inner
            .body_now
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        bump_peak(&self.inner.body_peak, now);
        Some(BodyPermit {
            inner: Arc::clone(&self.inner),
            slot,
            worker: slot / (BODY_SLOTS / WORKERS),
        })
    }

    /// Take a decode slot. `home` is a preference, not a requirement.
    ///
    /// The caller waits for the home worker for a bounded time, and then
    /// either decode thread takes the job. A busy home worker therefore
    /// cannot hold a preview while the other worker is idle.
    pub(super) fn acquire_decode(
        &self,
        home: Option<u8>,
        live: &mut dyn FnMut() -> bool,
    ) -> Option<DecodePermit> {
        self.acquire_decode_within(home, HOME_WAIT, live)
    }

    /// [`Self::acquire_decode`] with the home wait as an argument, so a test
    /// can hold a waiter on its home for longer than a slot return takes.
    fn acquire_decode_within(
        &self,
        home: Option<u8>,
        home_wait: Duration,
        live: &mut dyn FnMut() -> bool,
    ) -> Option<DecodePermit> {
        let want = home.map(usize::from);
        let deadline = std::time::Instant::now() + home_wait;
        let worker = wait_slot(
            &self.inner.decode_free,
            &self.inner.decode_ready,
            &self.inner.shut,
            live,
            Some(deadline),
            |slot| match want {
                Some(home) => slot == home || std::time::Instant::now() >= deadline,
                None => true,
            },
        )?;
        Some(DecodePermit {
            inner: Arc::clone(&self.inner),
            worker,
        })
    }
}

#[cfg(test)]
impl Scheduler {
    /// Stop both worker threads without marking the scheduler shut, so a
    /// slot is still granted and the job sent to it is dropped unsent.
    pub(super) fn stop_workers(&self) {
        for tx in &self.inner.txs {
            let _ = tx.send(Job::Stop);
        }
        let joins = {
            let mut guard = lock_mutex(&self.joins);
            std::mem::take(&mut *guard)
        };
        for handle in joins {
            let _ = handle.join();
        }
    }

    /// Mark the scheduler shut, so every later slot request is refused.
    pub(super) fn shut(&self) {
        self.inner.shut.store(true, Ordering::Release);
        self.inner.body_ready.notify_all();
        self.inner.decode_ready.notify_all();
    }

    /// Give `worker` a returned grey buffer of `bytes`, counted by the gate.
    pub(super) fn park_grey(&self, worker: usize, bytes: usize) -> bool {
        let gate = Arc::clone(&self.inner.gate);
        call_worker(&self.inner, worker, move || {
            let old = with_decode_workspace(|workspace| std::mem::take(&mut workspace.grey));
            gate.release_returned(old.capacity());
            drop(old);
            if !gate.count_returned(bytes) {
                return false;
            }
            with_decode_workspace(|workspace| workspace.grey = Vec::with_capacity(bytes));
            true
        })
        .unwrap_or(false)
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.inner.shut.store(true, Ordering::Release);
        self.inner.body_ready.notify_all();
        self.inner.decode_ready.notify_all();
        for tx in &self.inner.txs {
            let _ = tx.send(Job::Stop);
        }
        let joins = {
            let mut guard = lock_mutex(&self.joins);
            std::mem::take(&mut *guard)
        };
        for handle in joins {
            let _ = handle.join();
        }
    }
}

impl BodyPermit {
    pub(crate) fn worker(&self) -> u8 {
        u8::try_from(self.worker).unwrap_or(0)
    }

    pub(crate) fn retire(&self) -> Sender<Vec<u8>> {
        clone_retire(&self.inner, self.worker)
    }

    pub(crate) fn call<T: Send + 'static>(
        &self,
        job: impl FnOnce() -> T + Send + 'static,
    ) -> Option<T> {
        call_worker(&self.inner, self.worker, job)
    }
}

impl Drop for BodyPermit {
    fn drop(&mut self) {
        release_count(&self.inner.body_now);
        return_slot(&self.inner.body_free, &self.inner.body_ready, self.slot);
    }
}

impl DecodePermit {
    pub(crate) fn worker(&self) -> u8 {
        u8::try_from(self.worker).unwrap_or(0)
    }

    pub(crate) fn retire(&self) -> Sender<Vec<u8>> {
        clone_retire(&self.inner, self.worker)
    }

    pub(crate) fn call<T: Send + 'static>(
        &self,
        job: impl FnOnce() -> T + Send + 'static,
    ) -> Option<T> {
        call_worker(&self.inner, self.worker, job)
    }

    /// End the permit and keep the way home for the grey reply.
    ///
    /// A decode permit ends with the decode. The grey bytes are ready, so
    /// the slot is free again before the reply is written, and a client that
    /// stops reading holds no permit. `claim` is the reply's grey under the
    /// cap, and it travels with the reply until the write.
    pub(crate) fn finish(self, claim: GreyClaim) -> GreyHome {
        let home = GreyHome {
            inner: Arc::clone(&self.inner),
            worker: self.worker,
            claim,
        };
        drop(self);
        home
    }
}

/// The worker that allocated a finished decode's grey reply, and the reply's
/// grey bytes under the cap.
///
/// The permit has ended. Dropping this value unused releases the claim.
pub(crate) struct GreyHome {
    inner: Arc<Inner>,
    worker: usize,
    claim: GreyClaim,
}

impl GreyHome {
    /// Put the written grey reply back on the worker that allocated it.
    ///
    /// The job is queued and not awaited, so the connection thread never
    /// blocks on a worker that is busy decoding. When no request waits for
    /// grey room the worker keeps the buffer as its returned grey, in place
    /// of any it held, and the count moves with it. When one waits the
    /// worker frees the buffer and the claim is released after the free. If
    /// the worker is gone the job is dropped unsent, the grey is freed here
    /// as [`retire_bytes`] does for a dead worker, and its count is released.
    pub(crate) fn restore_grey(self, grey: Vec<u8>) {
        let Self {
            inner,
            worker,
            claim,
        } = self;
        let capacity = grey.capacity();
        let returned = inner.gate.return_reply(claim);
        let gate = Arc::clone(&inner.gate);
        let _ = post_job(&inner, worker, move || {
            reattach_grey(capacity);
            match returned {
                ReturnedGrey::Keep(kept) => {
                    let old = with_decode_workspace(|workspace| {
                        std::mem::replace(&mut workspace.grey, grey)
                    });
                    kept.place();
                    let old_bytes = old.capacity();
                    drop(old);
                    gate.release_returned(old_bytes);
                }
                ReturnedGrey::Free(claim) => {
                    drop(grey);
                    drop(claim);
                }
            }
        });
    }
}

/// Free this worker's returned grey buffer while a request waits for room.
fn trim_returned(gate: &GreyGate) -> bool {
    if !gate.has_waiters() {
        return false;
    }
    let grey = with_decode_workspace(|workspace| std::mem::take(&mut workspace.grey));
    let bytes = grey.capacity();
    drop(grey);
    gate.release_returned(bytes);
    bytes > 0
}

impl Drop for DecodePermit {
    fn drop(&mut self) {
        return_slot(
            &self.inner.decode_free,
            &self.inner.decode_ready,
            self.worker,
        );
    }
}

fn clone_retire(inner: &Inner, worker: usize) -> Sender<Vec<u8>> {
    match inner.retire.get(worker) {
        Some(sender) => sender.clone(),
        None => mpsc::channel().0,
    }
}

fn call_worker<T: Send + 'static>(
    inner: &Inner,
    worker: usize,
    job: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    let task: Box<dyn FnOnce() + Send> = Box::new(move || {
        let _ = tx.send(job());
    });
    let sender = inner.txs.get(worker)?;
    sender.send(Job::Run(task)).ok()?;
    rx.recv().ok()
}

/// Queue a job and do not wait for it. `false` means the worker is gone.
fn post_job(inner: &Inner, worker: usize, job: impl FnOnce() + Send + 'static) -> bool {
    let Some(sender) = inner.txs.get(worker) else {
        return false;
    };
    sender.send(Job::Run(Box::new(job))).is_ok()
}

/// Take a slot from `free` once `accept` allows it.
///
/// `deadline` bounds the wait for a slot the caller prefers: the sleep ends
/// at the time left, not at a whole `WAIT`, so a home preference is
/// abandoned on time. A waiter woken by a return that `accept` declines goes
/// back to sleep, and every waiter is woken on every return so a declined
/// return cannot starve the waiter that wanted that slot.
fn wait_slot(
    free: &Mutex<VecDeque<usize>>,
    ready: &Condvar,
    shut: &AtomicBool,
    live: &mut dyn FnMut() -> bool,
    deadline: Option<std::time::Instant>,
    accept: impl Fn(usize) -> bool,
) -> Option<usize> {
    let mut slots = lock_mutex(free);
    loop {
        if shut.load(Ordering::Acquire) {
            return None;
        }
        if let Some(pos) = slots.iter().position(|slot| accept(*slot)) {
            return slots.remove(pos);
        }
        if !live() {
            return None;
        }
        let wait = match deadline {
            Some(at) => {
                let left = at.saturating_duration_since(std::time::Instant::now());
                if left.is_zero() { WAIT } else { left.min(WAIT) }
            }
            None => WAIT,
        };
        let (guard, _) = ready
            .wait_timeout(slots, wait)
            .unwrap_or_else(|poison| poison.into_inner());
        slots = guard;
        if shut.load(Ordering::Acquire) || !live() {
            return None;
        }
    }
}

fn return_slot(free: &Mutex<VecDeque<usize>>, ready: &Condvar, slot: usize) {
    lock_mutex(free).push_back(slot);
    ready.notify_all();
}

fn release_count(slot: &AtomicUsize) {
    let mut current = slot.load(Ordering::Relaxed);
    loop {
        let next = current.saturating_sub(1);
        match slot.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn worker_loop(
    index: usize,
    rx: Receiver<Job>,
    retire_rx: Receiver<Vec<u8>>,
    inner: &Inner,
    started: &std::sync::Barrier,
) {
    lock_mutex(&inner.threads).insert(std::thread::current().id());
    started.wait();
    loop {
        drain_retired(&retire_rx);
        match rx.recv_timeout(WAIT) {
            Ok(Job::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if trim_returned(&inner.gate) {
                    publish(inner, index);
                }
            }
            Ok(Job::Run(job)) => {
                job();
                trim_returned(&inner.gate);
                publish(inner, index);
                drain_retired(&retire_rx);
            }
        }
    }
    drain_retired(&retire_rx);
    // The thread's workspace goes with the thread, so its returned grey
    // stops counting here.
    let grey = with_decode_workspace(|workspace| std::mem::take(&mut workspace.grey));
    let bytes = grey.capacity();
    drop(grey);
    inner.gate.release_returned(bytes);
}

fn drain_retired(rx: &Receiver<Vec<u8>>) {
    while let Ok(bytes) = rx.try_recv() {
        drop(bytes);
    }
}

fn publish(inner: &Inner, index: usize) {
    let Some(slot) = inner.accounted.get(index) else {
        return;
    };
    let total = WORKSPACE
        .with(|workspace| workspace_bytes(&workspace.borrow()))
        .saturating_add(DETACHED.with(Cell::get));
    slot.store(total, Ordering::Relaxed);
}

/// Scratch, native bytes and the grey reply for the worker running the closure.
pub(crate) fn with_decode_workspace<T>(body: impl FnOnce(&mut Workspace) -> T) -> T {
    WORKSPACE.with(|workspace| body(&mut workspace.borrow_mut()))
}

/// Count grey bytes that just left the workspace for a reply.
pub(crate) fn detach_grey(bytes: usize) {
    DETACHED.with(|cell| cell.set(cell.get().saturating_add(bytes)));
}

/// Grey bytes came back to the workspace, so they are detached no longer.
pub(crate) fn reattach_grey(bytes: usize) {
    DETACHED.with(|cell| cell.set(cell.get().saturating_sub(bytes)));
}

/// Send `bytes` to the worker that allocated them. A dead worker drops them here.
pub(crate) fn retire_bytes(tx: &Sender<Vec<u8>>, bytes: Vec<u8>) {
    if bytes.capacity() == 0 {
        return;
    }
    let _ = tx.send(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gate() -> Arc<GreyGate> {
        Arc::new(GreyGate::new(super::super::GREY_CAP))
    }

    #[test]
    fn four_body_slots_and_two_workers() -> Result<(), String> {
        let scheduler = Scheduler::start(test_gate());
        let mut held = Vec::new();
        for _ in 0..BODY_SLOTS {
            held.push(
                scheduler
                    .acquire_body(&mut || true)
                    .ok_or("a body slot was not granted")?,
            );
        }
        let (threads, peak, now) = scheduler.body_schedule();
        if threads != WORKERS || now != BODY_SLOTS || peak != BODY_SLOTS {
            return Err(format!(
                "bodies were threads {threads} peak {peak} now {now}"
            ));
        }
        let missed = scheduler.acquire_body(&mut || false);
        if missed.is_some() {
            return Err("a fifth body slot was granted".to_owned());
        }
        drop(held);
        let recorded = Arc::new(Mutex::new(HashSet::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        std::thread::scope(|scope| {
            for _ in 0..6 {
                let scheduler = &scheduler;
                let recorded = Arc::clone(&recorded);
                let errors = Arc::clone(&errors);
                scope.spawn(move || {
                    let outcome = (|| {
                        let permit = scheduler
                            .acquire_body(&mut || true)
                            .ok_or("a body slot was not granted".to_owned())?;
                        let seen = Arc::clone(&recorded);
                        permit
                            .call(move || {
                                seen.lock()
                                    .unwrap_or_else(|poison| poison.into_inner())
                                    .insert(std::thread::current().id());
                            })
                            .ok_or("the body job did not finish".to_owned())?;
                        Ok::<(), String>(())
                    })();
                    if let Err(error) = outcome {
                        errors
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .push(error);
                    }
                });
            }
        });
        let failures = lock_mutex(&errors);
        if let Some(error) = failures.first() {
            return Err(error.clone());
        }
        let seen = lock_mutex(&recorded).len();
        if seen != WORKERS {
            return Err(format!("body jobs ran on {seen} threads"));
        }
        Ok(())
    }

    #[test]
    fn decodes_stay_on_the_home_worker() -> Result<(), String> {
        let scheduler = Scheduler::start(test_gate());
        let first = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("worker 0 was not granted")?;
        if first.worker() != 0 {
            return Err(format!("the home worker was {}", first.worker()));
        }
        let second = scheduler
            .acquire_decode(Some(1), &mut || true)
            .ok_or("worker 1 was not granted")?;
        if second.worker() != 1 {
            return Err(format!("the other worker was {}", second.worker()));
        }
        let missed = scheduler.acquire_decode(Some(0), &mut || false);
        if missed.is_some() {
            return Err("a second decode was granted on worker 0".to_owned());
        }
        let home = first
            .call(|| std::thread::current().id())
            .ok_or("worker 0 did not answer")?;
        let other = second
            .call(|| std::thread::current().id())
            .ok_or("worker 1 did not answer")?;
        if home == other {
            return Err("both decodes ran on one thread".to_owned());
        }
        let retire = first.retire();
        let (tx, rx) = mpsc::channel();
        let sender = tx.clone();
        // Retiring from another thread must not deadlock the home worker.
        let _ = std::thread::spawn(move || {
            retire_bytes(&retire, {
                let mut bytes = Vec::new();
                bytes.reserve_exact(64);
                bytes
            });
            let _ = sender.send(());
        })
        .join();
        let _ = rx.recv_timeout(Duration::from_secs(2));
        // The next job on worker 0 runs after the retired buffer is dropped.
        let after = first
            .call(|| std::thread::current().id())
            .ok_or("worker 0 did not answer again")?;
        if after != home {
            return Err("the retire drain did not stay on worker 0".to_owned());
        }
        drop(tx);
        drop(first);
        drop(second);
        Ok(())
    }

    #[test]
    fn a_busy_home_worker_hands_the_job_to_the_other_worker() -> Result<(), String> {
        let scheduler = Scheduler::start(test_gate());
        let first = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("worker 0 was not granted")?;
        if first.worker() != 0 {
            return Err("worker 0 was not the home worker".to_owned());
        }
        let started = std::time::Instant::now();
        let fallback = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("no slot was granted after the home wait")?;
        if fallback.worker() != 1 {
            return Err(format!("the fallback ran on {}", fallback.worker()));
        }
        let elapsed = started.elapsed();
        if elapsed < HOME_WAIT {
            return Err("the fallback did not wait for the home worker".to_owned());
        }
        // The wait ends at the time left, not at a whole WAIT. The fallback
        // takes about HOME_WAIT, not ten times that.
        if elapsed > HOME_WAIT + Duration::from_millis(20) {
            return Err(format!(
                "the fallback took {:.1} ms, about ten times the home wait",
                elapsed.as_secs_f64() * 1000.0
            ));
        }
        drop(first);
        drop(fallback);
        Ok(())
    }

    /// A slot that returns while a waiter declines it still reaches a waiter
    /// that accepts it.
    ///
    /// Four waiters want worker 0 for two seconds, so they decline worker 1.
    /// They wait first. A waiter with no home waits last. When worker 1
    /// returns, waking only one waiter wakes a decliner, and the waiter with
    /// no home sleeps a whole `WAIT` beside a free slot.
    #[test]
    fn a_returned_slot_reaches_a_waiter_that_accepts_it() -> Result<(), String> {
        let scheduler = Scheduler::start(test_gate());
        let on_zero = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("worker 0 was not granted")?;
        let on_one = scheduler
            .acquire_decode(Some(1), &mut || true)
            .ok_or("worker 1 was not granted")?;
        let stop = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let mut decliners = Vec::new();
            for _ in 0..4 {
                let scheduler = &scheduler;
                let stop = &stop;
                decliners.push(scope.spawn(move || {
                    let permit = scheduler.acquire_decode_within(
                        Some(0),
                        Duration::from_secs(2),
                        &mut || !stop.load(Ordering::Acquire),
                    );
                    drop(permit);
                }));
            }
            std::thread::sleep(Duration::from_millis(20));
            let (tx, rx) = mpsc::channel();
            let shared = &scheduler;
            let accepter = scope.spawn(move || {
                let permit = shared.acquire_decode(None, &mut || true);
                let _ = tx.send(std::time::Instant::now());
                permit.map(|permit| permit.worker())
            });
            std::thread::sleep(Duration::from_millis(20));
            let returned = std::time::Instant::now();
            drop(on_one);
            let granted = rx.recv_timeout(Duration::from_secs(2));
            let worker = accepter
                .join()
                .map_err(|_| "the waiter panicked".to_owned());
            stop.store(true, Ordering::Release);
            drop(on_zero);
            for decliner in decliners {
                let _ = decliner.join();
            }
            let granted = granted.map_err(|_| "the waiter was never granted".to_owned())?;
            if worker? != Some(1) {
                return Err("the waiter did not take the returned slot".to_owned());
            }
            let waited = granted.saturating_duration_since(returned);
            if waited > Duration::from_millis(25) {
                return Err(format!(
                    "the waiter sat {:.1} ms beside a free slot",
                    waited.as_secs_f64() * 1000.0
                ));
            }
            Ok::<(), String>(())
        })
    }

    /// Shutdown wakes a waiter that would otherwise wait for its home.
    #[test]
    fn a_waiting_decode_ends_at_shutdown() -> Result<(), String> {
        let scheduler = Scheduler::start(test_gate());
        let on_zero = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("worker 0 was not granted")?;
        let on_one = scheduler
            .acquire_decode(Some(1), &mut || true)
            .ok_or("worker 1 was not granted")?;
        let started = std::time::Instant::now();
        let ended = std::thread::scope(|scope| {
            let waiter = scope.spawn(|| scheduler.acquire_decode(Some(0), &mut || true).is_some());
            std::thread::sleep(Duration::from_millis(20));
            scheduler.inner.shut.store(true, Ordering::Release);
            scheduler.inner.decode_ready.notify_all();
            waiter.join().map_err(|_| "the waiter panicked".to_owned())
        })?;
        if ended {
            return Err("a decode was granted after shutdown".to_owned());
        }
        if started.elapsed() > Duration::from_millis(500) {
            return Err("the waiter did not end at shutdown".to_owned());
        }
        drop(on_zero);
        drop(on_one);
        Ok(())
    }

    /// A written reply kept by a worker that already keeps a buffer replaces
    /// it, and the replaced buffer stops counting.
    #[test]
    fn a_kept_reply_replaces_the_buffer_its_worker_kept() -> Result<(), String> {
        let gate = test_gate();
        let scheduler = Scheduler::start(Arc::clone(&gate));
        if !scheduler.park_grey(0, 1000) {
            return Err("the worker kept no grey".to_owned());
        }
        let permit = scheduler
            .acquire_decode(Some(0), &mut || true)
            .ok_or("no decode slot")?;
        let claim = gate.reserve(5000).ok_or("no grey room")?;
        let home = permit.finish(claim);
        home.restore_grey(Vec::with_capacity(5000));
        let started = std::time::Instant::now();
        while gate.held() != (0, 5000) {
            if started.elapsed() > Duration::from_secs(5) {
                return Err(format!("the gate held {:?}", gate.held()));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(scheduler);
        // The workers are gone, and their kept buffers with them.
        if gate.held() != (0, 0) {
            return Err(format!("after shutdown the gate held {:?}", gate.held()));
        }
        Ok(())
    }
}
