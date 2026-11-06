//! Two long-lived decode workers. Every decode the process server runs is a
//! job here, including a preview that carries the file's bytes, and every
//! pooled request body is read and freed on these two threads.
//!
//! `P` in the resident formula is 2, so there are two threads. The `4` in
//! `4 * S_max` is the most request bodies those threads keep at once. The
//! count does not follow the host core count. A pooled body is freed by the
//! worker that read it. A decoded frame's sample buffer that leaves the
//! cache is parked on the free list only when cached bytes plus free bytes
//! fit the budget, and is freed otherwise.

use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use super::{Workspace, bump_peak, lock_mutex, workspace_bytes};

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
    pub(super) fn start() -> Self {
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
        let want = home.map(usize::from);
        let deadline = std::time::Instant::now() + HOME_WAIT;
        let worker = wait_slot(
            &self.inner.decode_free,
            &self.inner.decode_ready,
            &self.inner.shut,
            live,
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
    /// stops reading holds no permit.
    pub(crate) fn finish(self) -> GreyHome {
        let home = GreyHome {
            inner: Arc::clone(&self.inner),
            worker: self.worker,
        };
        drop(self);
        home
    }
}

/// The worker that allocated a finished decode's grey reply.
///
/// The permit has ended. This value only carries the worker, so the reply
/// can be returned after it is written.
pub(crate) struct GreyHome {
    inner: Arc<Inner>,
    worker: usize,
}

impl GreyHome {
    /// Put the grey reply back on the worker that allocated it.
    ///
    /// The job is queued and not awaited, so the connection thread never
    /// blocks on a worker that is busy decoding. If that worker is gone the
    /// grey is dropped here, as [`retire_bytes`] does for a dead worker.
    pub(crate) fn restore_grey(self, grey: Vec<u8>) {
        let capacity = grey.capacity();
        // The closure owns the grey. If the worker is gone the job is
        // dropped unsent and the grey is dropped with it.
        let _ = post_job(&self.inner, self.worker, move || {
            with_decode_workspace(|workspace| workspace.grey = grey);
            reattach_grey(capacity);
        });
        drop(self);
    }
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

fn wait_slot(
    free: &Mutex<VecDeque<usize>>,
    ready: &Condvar,
    shut: &AtomicBool,
    live: &mut dyn FnMut() -> bool,
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
        let (guard, _) = ready
            .wait_timeout(slots, WAIT)
            .unwrap_or_else(|poison| poison.into_inner());
        slots = guard;
        if shut.load(Ordering::Acquire) || !live() {
            return None;
        }
    }
}

fn return_slot(free: &Mutex<VecDeque<usize>>, ready: &Condvar, slot: usize) {
    lock_mutex(free).push_back(slot);
    ready.notify_one();
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
            Err(RecvTimeoutError::Timeout) => {}
            Ok(Job::Run(job)) => {
                job();
                publish(inner, index);
                drain_retired(&retire_rx);
            }
        }
    }
    drain_retired(&retire_rx);
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

    #[test]
    fn four_body_slots_and_two_workers() -> Result<(), String> {
        let scheduler = Scheduler::start();
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
        let scheduler = Scheduler::start();
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
        let scheduler = Scheduler::start();
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
        if started.elapsed() < HOME_WAIT {
            return Err("the fallback did not wait for the home worker".to_owned());
        }
        drop(first);
        drop(fallback);
        Ok(())
    }
}
