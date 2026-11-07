//! Files and decoded frames held by one viewer process.
//!
//! One `Lru` accounts both. Encoded bytes leave by `Lru::remove` once every
//! frame of that file is held. The tier is `CacheTier::Decoded` because the
//! frames are what the budget is for. A decoded frame's sample buffer that
//! leaves the cache is parked on a free list keyed by its exact byte length
//! when cached bytes plus free bytes fit the budget, and is freed otherwise.
//! Cached frames and free buffers share one budget. A decode of that length
//! takes a parked buffer when there is one, and allocates otherwise.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Duration;

use ocelli_cache::{Admission, Budgeted, CacheTier, Lru, Pressure};
use ocelli_core::Stored;

use crate::preview::HeldFrame;

#[path = "schedule.rs"]
mod schedule;

pub(crate) use schedule::with_decode_workspace;
pub(crate) use schedule::{BodyPermit, DecodePermit, GreyHome, detach_grey, retire_bytes};

const DEFAULT_CACHE_BYTES: usize = 256 * 1024 * 1024;
const CONNECTION_CAP: usize = 32;
const PERMITS: usize = 2;

/// Bytes the viewer may hold.
pub struct ViewerConfig {
    /// Budget in bytes. Zero admits nothing.
    pub cache_bytes: usize,
}

impl ViewerConfig {
    /// Read `OCELLI_CACHE_MIB`. Unset means 256 MiB.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` when the variable is not a whole number of
    /// mebibytes, or when that count does not fit the address size.
    pub fn from_env() -> std::io::Result<Self> {
        match std::env::var("OCELLI_CACHE_MIB") {
            Ok(text) => Ok(Self {
                cache_bytes: cache_mib_to_bytes(&text)?,
            }),
            Err(std::env::VarError::NotPresent) => Ok(Self {
                cache_bytes: DEFAULT_CACHE_BYTES,
            }),
            Err(std::env::VarError::NotUnicode(_)) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "OCELLI_CACHE_MIB is not a whole number",
            )),
        }
    }
}

fn cache_mib_to_bytes(text: &str) -> std::io::Result<usize> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "OCELLI_CACHE_MIB is not a whole number",
        ));
    }
    let mib = trimmed.parse::<u64>().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "OCELLI_CACHE_MIB is not a whole number",
        )
    })?;
    let bytes = mib.checked_mul(1024 * 1024).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "OCELLI_CACHE_MIB is too large",
        )
    })?;
    usize::try_from(bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "OCELLI_CACHE_MIB is too large",
        )
    })
}

/// `<generation>.<serial>` in lowercase hex.
///
/// The generation is fixed for one `Viewer`. The serial is never reused.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FileId {
    pub(crate) generation: u64,
    pub(crate) serial: u64,
}

impl FileId {
    /// Parse lowercase hex. Anything else is absent.
    ///
    /// A leading `+` and a leading zero are absent too. `from_str_radix`
    /// accepts both, and either would make `+a.+b` or `0a.b` name `a.b`.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (left, right) = text.split_once('.')?;
        if !hex_id_part(left) || !hex_id_part(right) {
            return None;
        }
        let generation = u64::from_str_radix(left, 16).ok()?;
        let serial = u64::from_str_radix(right, 16).ok()?;
        Some(Self { generation, serial })
    }

    pub(crate) fn header_value(self) -> String {
        format!("{:x}.{:x}", self.generation, self.serial)
    }
}

fn has_upper_hex(text: &str) -> bool {
    text.bytes().any(|byte| byte.is_ascii_uppercase())
}

fn hex_id_part(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || has_upper_hex(text) {
        return false;
    }
    // "0" is a value. "0a" and "00" are the same value written longer.
    if bytes.len() > 1 && bytes.first() == Some(&b'0') {
        return false;
    }
    bytes.iter().all(u8::is_ascii_hexdigit)
}

/// What one lookup found. The cache lock is not held afterwards.
#[cfg(test)]
pub(crate) enum Lookup {
    /// The id is unknown, or the bytes and this frame are both gone.
    Gone,
    /// The decoded frame.
    Hit(Arc<CachedFrame>),
    /// The posted bytes, shared with the cache so a decode does not copy them.
    Miss(Arc<PostedBytes>),
}

/// A preview's view of one id. The posted bytes stay in the cache for the
/// whole decode, so a second preview or `/headers` still finds them.
pub(crate) enum PreviewSource {
    /// The id is unknown, or the bytes and this frame are both gone.
    Gone,
    /// The decoded frame.
    Hit(Arc<CachedFrame>),
    /// Posted bytes. The cache keeps its own `Arc` until the frame is stored.
    Shared(SharedBytes),
}

impl PreviewSource {
    /// The decode's place in its id's in-flight count, when bytes were
    /// handed out. See [`SharedBytes::take_flight`].
    pub(crate) fn take_flight(&mut self) -> Option<Flight> {
        match self {
            Self::Shared(shared) => shared.take_flight(),
            Self::Gone | Self::Hit(_) => None,
        }
    }
}

/// Posted bytes handed to one decode, and that decode's place in the id's
/// in-flight count.
pub(crate) struct SharedBytes {
    bytes: Arc<PostedBytes>,
    flight: Option<Flight>,
}

impl SharedBytes {
    /// The decode's place in the count. The caller keeps it until the frame
    /// is stored or the decode is given up, so a forget made meanwhile is not
    /// undone by the store. Left here, it is released when the bytes drop.
    pub(crate) fn take_flight(&mut self) -> Option<Flight> {
        self.flight.take()
    }
}

impl std::ops::Deref for SharedBytes {
    type Target = PostedBytes;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

/// One decode of an id whose posted bytes `preview_source` handed out.
///
/// Drop takes it out of the id's count. The last one out for an id that was
/// forgotten meanwhile takes the id out of the doomed set, so both stay
/// bounded by the decodes in flight. Never dropped under the cache lock.
pub(crate) struct Flight {
    cache: Arc<Mutex<CacheState>>,
    id: FileId,
}

impl Drop for Flight {
    fn drop(&mut self) {
        lock_mutex(&self.cache).land(self.id);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HeldKey {
    File(FileId),
    Frame(FileId, u32),
}

enum Held {
    /// Posted bytes. Shared, so a lookup does not copy the buffer.
    File(Arc<PostedBytes>),
    Frame(Arc<CachedFrame>),
}

/// Posted bytes, and the worker that should free them.
pub(crate) struct PostedBytes {
    bytes: Vec<u8>,
    worker: Option<u8>,
    retire: Option<Sender<Vec<u8>>>,
}

impl PostedBytes {
    #[cfg(test)]
    fn plain(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            worker: None,
            retire: None,
        }
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn worker(&self) -> Option<u8> {
        self.worker
    }

    fn capacity(&self) -> usize {
        self.bytes.capacity()
    }
}

impl Drop for PostedBytes {
    fn drop(&mut self) {
        let bytes = std::mem::take(&mut self.bytes);
        if let Some(tx) = self.retire.take() {
            schedule::retire_bytes(&tx, bytes);
        }
    }
}

impl std::ops::Deref for PostedBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

/// A cached frame. When it leaves the cache its sample buffer is parked on
/// the free list only if the budget still fits it, and is freed otherwise.
pub(crate) struct CachedFrame {
    inner: HeldFrame,
    worker: Option<u8>,
    retire: Option<Sender<Vec<u8>>>,
    recycle: Option<Sender<Vec<u8>>>,
}

impl CachedFrame {
    #[cfg(test)]
    fn plain(inner: HeldFrame) -> Self {
        Self {
            inner,
            worker: None,
            retire: None,
            recycle: None,
        }
    }

    pub(crate) fn worker(&self) -> Option<u8> {
        self.worker
    }
}

impl std::ops::Deref for CachedFrame {
    type Target = HeldFrame;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for CachedFrame {
    fn drop(&mut self) {
        let samples = std::mem::take(&mut self.inner.samples);
        if samples.capacity() == 0 {
            return;
        }
        if let Some(tx) = self.recycle.take() {
            let _ = tx.send(samples);
            return;
        }
        if let Some(tx) = self.retire.take() {
            schedule::retire_bytes(&tx, samples);
        }
    }
}

impl Budgeted for Held {
    fn bytes(&self) -> usize {
        match self {
            Self::File(bytes) => bytes.capacity(),
            Self::Frame(frame) => frame.charged_bytes(),
        }
    }
}

struct TrackedFrame {
    key: HeldKey,
    len: usize,
}

struct CacheState {
    lru: Lru<HeldKey, Held>,
    files: usize,
    frames: usize,
    /// Sample buffers keyed by exact byte length.
    free: HashMap<usize, Vec<Vec<u8>>>,
    free_bytes: usize,
    limit: usize,
    /// Oldest frame at the front. A hit moves its key to the back.
    frame_order: VecDeque<TrackedFrame>,
    recycle_rx: Receiver<Vec<u8>>,
    /// Decodes holding each id's posted bytes, counted where
    /// `preview_source` hands them out and released when the decode's
    /// [`Flight`] drops after its store.
    in_flight: BTreeMap<FileId, u32>,
    /// Ids forgotten while a decode held their bytes. `store_frame_with`
    /// stores nothing for them. An id leaves when its count reaches zero.
    doomed: BTreeSet<FileId>,
    #[cfg(test)]
    reused: usize,
    #[cfg(test)]
    fresh: usize,
}

/// Cloneable handle so a decode worker can take a free buffer.
#[derive(Clone)]
pub(crate) struct CacheHandle {
    inner: Arc<Mutex<CacheState>>,
}

/// One decode permit. The scratch, the native byte copy and the grey reply
/// stay with the permit and grow to the largest frame that permit has served.
#[derive(Default)]
pub(crate) struct Workspace {
    pub(crate) scratch: Vec<Stored>,
    /// Little-endian or big-endian bytes copied from a `U16` pixel value.
    pub(crate) native: Vec<u8>,
    /// One display byte per sample. Moved into the reply, then returned.
    pub(crate) grey: Vec<u8>,
}

struct PoolState {
    idle: Vec<Workspace>,
    spare_grey: Vec<Vec<u8>>,
}

struct PermitPool {
    idle: Mutex<PoolState>,
    ready: Condvar,
    /// Scratch, native and grey bytes on permits that are not idle.
    out: AtomicUsize,
}

pub(crate) struct WorkspaceGuard<'a> {
    pool: &'a PermitPool,
    pub(crate) workspace: Workspace,
    charged: usize,
}

impl WorkspaceGuard<'_> {
    pub(crate) fn note_growth(&mut self) {
        let now = workspace_bytes(&self.workspace);
        if now > self.charged {
            self.pool
                .out
                .fetch_add(now - self.charged, Ordering::Relaxed);
            self.charged = now;
        }
    }
}

impl Drop for WorkspaceGuard<'_> {
    fn drop(&mut self) {
        self.note_growth();
        self.pool.out.fetch_sub(self.charged, Ordering::Relaxed);
        self.charged = 0;
        let workspace = std::mem::take(&mut self.workspace);
        let mut state = lock_mutex(&self.pool.idle);
        state.idle.push(workspace);
        self.pool.ready.notify_one();
    }
}

fn workspace_bytes(workspace: &Workspace) -> usize {
    workspace
        .scratch
        .capacity()
        .saturating_mul(size_of::<Stored>())
        .saturating_add(workspace.native.capacity())
        .saturating_add(workspace.grey.capacity())
}

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// `N_max` in the resident formula, in pixels: the 3000 by 2900 frame the
/// plan's bound of 528,194,192 bytes is computed for.
pub(crate) const N_MAX_PIXELS: usize = 3000 * 2900;

/// The grey cap in bytes, the resident bound's grey term `P * N_max` at one
/// grey byte per pixel. A reply's grey counts from admission until it is
/// written, and a worker's kept grey counts for as long as it is kept.
pub(crate) const GREY_CAP: usize = PERMITS * N_MAX_PIXELS;

/// Every grey byte the process server holds, under one fixed cap.
///
/// Two kinds of grey buffer count. A reply that is not yet written holds its
/// grey from the moment its render is admitted, by capacity. Each decode
/// worker keeps at most one returned grey buffer to render into next. Their
/// sum never exceeds the cap, which is [`GREY_CAP`] for the server. A render
/// whose grey would exceed it waits.
///
/// Waiting requests are admitted in arrival order. A request takes a place in
/// one queue when its admission is first refused, and the refusal hands it
/// that place. A waiting request is admitted only at the head of the queue
/// and only when its grey fits. A request that holds no room is admitted at
/// once only when it fits and the queue is empty. A request that already
/// holds room is never queued behind anything.
///
/// A grey larger than the whole cap, a frame of more than [`GREY_CAP`] pixels,
/// draws alone. It is admitted only when the gate holds no other grey, and
/// while it is held nothing else fits, so grey bytes stay within the larger
/// of the cap and that one grey. Its buffer is never kept by a worker after
/// the write.
pub(crate) struct GreyGate {
    state: Mutex<GreyState>,
    ready: Condvar,
    cap: usize,
}

struct GreyState {
    /// Grey bytes held by replies from admission to write, by capacity.
    replies: usize,
    /// Grey bytes in the decode workers' returned buffers.
    returned: usize,
    /// Tickets of the requests waiting for room, oldest first. While any
    /// waits, no returned buffer is kept.
    queue: VecDeque<u64>,
    /// The ticket the next waiting request takes.
    next_ticket: u64,
    /// The largest `used` seen since the gate was made or last reset.
    #[cfg(test)]
    peak: usize,
}

impl GreyState {
    fn used(&self) -> usize {
        self.replies.saturating_add(self.returned)
    }

    fn note_peak(&mut self) {
        #[cfg(test)]
        {
            self.peak = self.peak.max(self.used());
        }
    }

    /// Take the next place at the back of the queue.
    fn join_queue(&mut self) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.queue.push_back(ticket);
        ticket
    }
}

/// Grey bytes one request holds under the cap. Drop releases them.
pub(crate) struct GreyClaim {
    gate: Arc<GreyGate>,
    bytes: usize,
}

impl GreyClaim {
    /// Hand the bytes to the caller's accounting without releasing them.
    fn disarm(&mut self) -> usize {
        std::mem::take(&mut self.bytes)
    }
}

impl Drop for GreyClaim {
    fn drop(&mut self) {
        let bytes = self.disarm();
        if bytes > 0 {
            self.gate.release_replies(bytes);
        }
    }
}

/// A written reply's grey on its way back to its worker.
pub(crate) enum ReturnedGrey {
    /// No request waits, so the worker keeps the buffer. The bytes moved from
    /// the reply count to the returned count.
    Keep(KeptGrey),
    /// A request waits, so the worker frees the buffer. The bytes still count
    /// until the claim drops after the free.
    Free(GreyClaim),
}

/// Returned grey bytes not yet in a worker's workspace. Drop releases them
/// unless the worker took the buffer with [`KeptGrey::place`].
pub(crate) struct KeptGrey {
    gate: Arc<GreyGate>,
    bytes: usize,
}

impl KeptGrey {
    /// The worker's workspace holds the buffer now, and the count with it.
    pub(crate) fn place(mut self) {
        self.bytes = 0;
    }
}

impl Drop for KeptGrey {
    fn drop(&mut self) {
        let bytes = std::mem::take(&mut self.bytes);
        self.gate.release_returned(bytes);
    }
}

/// One request's place in the queue for grey room. Drop leaves the queue and
/// wakes the waiters, so the next one may be at the head now.
pub(crate) struct GreyWaiter {
    gate: Arc<GreyGate>,
    ticket: u64,
    /// Grey bytes the request waits for.
    len: usize,
}

impl GreyWaiter {
    /// Grey bytes the request waits for.
    pub(crate) fn grey_len(&self) -> usize {
        self.len
    }
}

impl Drop for GreyWaiter {
    fn drop(&mut self) {
        let mut state = lock_mutex(&self.gate.state);
        if let Some(at) = state.queue.iter().position(|ticket| *ticket == self.ticket) {
            let _ = state.queue.remove(at);
        }
        drop(state);
        self.gate.ready.notify_all();
    }
}

impl GreyGate {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            state: Mutex::new(GreyState {
                replies: 0,
                returned: 0,
                queue: VecDeque::new(),
                next_ticket: 0,
                #[cfg(test)]
                peak: 0,
            }),
            ready: Condvar::new(),
            cap,
        }
    }

    /// Whether `len` grey bytes fit beside `others` held now.
    ///
    /// A grey larger than the cap fits only when nothing else is held. Any
    /// other grey fits when the sum fits the cap.
    fn size_fits(&self, others: usize, len: usize) -> bool {
        if len > self.cap {
            return others == 0;
        }
        others.saturating_add(len) <= self.cap
    }

    /// Whether a request that is not waiting may enter with `len` bytes.
    ///
    /// A request that already holds room (`holds_room`) is never queued
    /// behind anything. Any other request enters only when the queue is
    /// empty, so it cannot pass a request that arrived before it.
    fn fits(&self, state: &GreyState, others: usize, len: usize, holds_room: bool) -> bool {
        if !holds_room && !state.queue.is_empty() {
            return false;
        }
        self.size_fits(others, len)
    }

    /// Reserve `len` grey bytes for a render that has not run yet, as a
    /// request that holds no room and is not waiting.
    ///
    /// `None` means it may not enter now. It takes no place in the queue.
    #[cfg(test)]
    pub(crate) fn reserve(self: &Arc<Self>, len: usize) -> Option<GreyClaim> {
        let mut state = lock_mutex(&self.state);
        if !self.fits(&state, state.used(), len, false) {
            return None;
        }
        state.replies = state.replies.saturating_add(len);
        state.note_peak();
        Some(GreyClaim {
            gate: Arc::clone(self),
            bytes: len,
        })
    }

    /// Admit the waiter when it is at the head of the queue and its grey
    /// fits. The admission takes it out of the queue.
    fn take_turn(
        self: &Arc<Self>,
        state: &mut GreyState,
        waiter: &GreyWaiter,
    ) -> Option<GreyClaim> {
        if state.queue.front() != Some(&waiter.ticket) || !self.size_fits(state.used(), waiter.len)
        {
            return None;
        }
        let _ = state.queue.pop_front();
        state.replies = state.replies.saturating_add(waiter.len);
        state.note_peak();
        Some(GreyClaim {
            gate: Arc::clone(self),
            bytes: waiter.len,
        })
    }

    /// Admit `waiter` now, or sleep until a release, a departure from the
    /// queue, or `slice`, and try once more.
    ///
    /// The check and the sleep hold one lock, so a wake between them is not
    /// lost. An admission wakes the others, since the next in the queue may
    /// fit too.
    pub(crate) fn wait_turn(
        self: &Arc<Self>,
        waiter: &GreyWaiter,
        slice: Duration,
    ) -> Option<GreyClaim> {
        let mut state = lock_mutex(&self.state);
        let mut claim = self.take_turn(&mut state, waiter);
        if claim.is_none() {
            state = self
                .ready
                .wait_timeout(state, slice)
                .unwrap_or_else(|poison| poison.into_inner())
                .0;
            claim = self.take_turn(&mut state, waiter);
        }
        drop(state);
        if claim.is_some() {
            self.ready.notify_all();
        }
        claim
    }

    /// Admit a render of `len` grey bytes into a worker buffer of `kept`
    /// bytes, which the returned count holds.
    ///
    /// The render reuses the buffer when it is large enough and replaces it
    /// otherwise, so the reply then holds `max(kept, len)` bytes and the
    /// buffer is no longer returned. `claim` is room this request already
    /// waited for, and it counts toward the admission. A refusal releases it
    /// and hands back a place at the back of the queue for `len` bytes.
    /// A grey larger than the cap is admitted only when no other grey is held.
    pub(crate) fn admit(
        self: &Arc<Self>,
        claim: Option<GreyClaim>,
        len: usize,
        kept: usize,
    ) -> Result<GreyClaim, GreyWaiter> {
        let mut claim = claim.unwrap_or(GreyClaim {
            gate: Arc::clone(self),
            bytes: 0,
        });
        let charged = kept.max(len);
        let mut state = lock_mutex(&self.state);
        let others = state
            .used()
            .saturating_sub(claim.bytes)
            .saturating_sub(kept);
        if !self.fits(&state, others, charged, claim.bytes > 0) {
            let ticket = state.join_queue();
            drop(state);
            drop(claim);
            return Err(GreyWaiter {
                gate: Arc::clone(self),
                ticket,
                len,
            });
        }
        let after = others.saturating_add(charged);
        let before = state.used();
        let held = claim.disarm();
        state.replies = state.replies.saturating_sub(held).saturating_add(charged);
        state.returned = state.returned.saturating_sub(kept);
        state.note_peak();
        drop(state);
        claim.bytes = charged;
        // Reusing a buffer for a smaller grey than the room waited for
        // frees room another request may be waiting for.
        if after < before {
            self.ready.notify_all();
        }
        Ok(claim)
    }

    /// The render finished. The reply holds `reply` bytes and the worker's
    /// buffer is `kept` bytes, both inside what the claim was charged.
    pub(crate) fn settle(&self, claim: &mut GreyClaim, reply: usize, kept: usize) {
        debug_assert!(
            reply.saturating_add(kept) <= claim.bytes,
            "a render held more grey than it was admitted for"
        );
        let mut state = lock_mutex(&self.state);
        state.replies = state
            .replies
            .saturating_sub(claim.bytes)
            .saturating_add(reply);
        state.returned = state.returned.saturating_add(kept);
        drop(state);
        let freed = claim.bytes > reply.saturating_add(kept);
        claim.bytes = reply;
        if freed {
            self.ready.notify_all();
        }
    }

    /// The reply was written. Keep its buffer as returned, or free it when a
    /// request waits for room or the buffer is larger than the cap.
    pub(crate) fn return_reply(&self, mut claim: GreyClaim) -> ReturnedGrey {
        let mut state = lock_mutex(&self.state);
        if !state.queue.is_empty() || claim.bytes > self.cap {
            drop(state);
            return ReturnedGrey::Free(claim);
        }
        let bytes = claim.disarm();
        state.replies = state.replies.saturating_sub(bytes);
        state.returned = state.returned.saturating_add(bytes);
        drop(state);
        ReturnedGrey::Keep(KeptGrey {
            gate: Arc::clone(&claim.gate),
            bytes,
        })
    }

    fn release_replies(&self, bytes: usize) {
        let mut state = lock_mutex(&self.state);
        state.replies = state.replies.saturating_sub(bytes);
        drop(state);
        self.ready.notify_all();
    }

    /// A worker freed a returned buffer of `bytes`.
    pub(crate) fn release_returned(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let mut state = lock_mutex(&self.state);
        state.returned = state.returned.saturating_sub(bytes);
        drop(state);
        self.ready.notify_all();
    }

    /// Whether a request waits for room, so returned buffers should go.
    pub(crate) fn has_waiters(&self) -> bool {
        !lock_mutex(&self.state).queue.is_empty()
    }

    /// Take a place at the back of the queue for `len` grey bytes, as a
    /// refused admission does.
    #[cfg(test)]
    pub(crate) fn enter_wait(self: &Arc<Self>, len: usize) -> GreyWaiter {
        let ticket = lock_mutex(&self.state).join_queue();
        GreyWaiter {
            gate: Arc::clone(self),
            ticket,
            len,
        }
    }

    /// The cap in bytes.
    pub(crate) fn cap(&self) -> usize {
        self.cap
    }

    /// The largest grey total held at once since the last reset.
    #[cfg(test)]
    pub(crate) fn peak(&self) -> usize {
        lock_mutex(&self.state).peak
    }

    /// Start the peak again from the grey held now.
    #[cfg(test)]
    pub(crate) fn reset_peak(&self) {
        let mut state = lock_mutex(&self.state);
        state.peak = state.used();
    }

    /// Grey bytes held by unwritten replies, then by returned buffers.
    pub(crate) fn held(&self) -> (usize, usize) {
        let state = lock_mutex(&self.state);
        (state.replies, state.returned)
    }

    /// Requests waiting for room now.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> usize {
        lock_mutex(&self.state).queue.len()
    }

    /// Count a returned buffer of `bytes` a test placed, when it fits.
    #[cfg(test)]
    pub(crate) fn count_returned(&self, bytes: usize) -> bool {
        let mut state = lock_mutex(&self.state);
        if state.used().saturating_add(bytes) > self.cap {
            return false;
        }
        state.returned = state.returned.saturating_add(bytes);
        state.note_peak();
        true
    }
}

fn bump_peak(slot: &AtomicUsize, value: usize) {
    let mut current = slot.load(Ordering::Relaxed);
    while value > current {
        match slot.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

/// One reading of the viewer's pixel bytes.
pub(crate) struct MemoryCensus {
    /// Configured cache size and bytes the LRU holds.
    pub(crate) pressure: Pressure,
    /// Posted files in the cache.
    pub(crate) files: usize,
    /// Decoded frames in the cache.
    pub(crate) frames: usize,
    /// Body bytes being read and decode sample bytes not yet in the cache.
    pub(crate) inflight: usize,
    /// Every grey byte under the cap: unwritten replies and returned buffers.
    pub(crate) grey: usize,
    /// The part of `grey` held by replies that are not yet written.
    pub(crate) grey_replies: usize,
    /// Permit scratch, native copies, grey replies and returned grey buffers.
    pub(crate) workspace: usize,
    /// Decoded sample bytes on the free list.
    pub(crate) free: usize,
}

/// What one forget removed. `bytes` is the admitted size of every entry that
/// left, so `GET /held`'s `used` drops by exactly that.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Forgotten {
    /// Posted files removed.
    pub(crate) files: usize,
    /// Decoded frames removed.
    pub(crate) frames: usize,
    /// Bytes the removed entries were admitted at.
    pub(crate) bytes: usize,
    /// Ids that held nothing.
    pub(crate) unknown: usize,
}

/// One inflight snapshot taken on the request path.
#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct InflightStep {
    /// The point in the request.
    pub(crate) label: &'static str,
    /// `MemoryCensus::inflight` at that point.
    pub(crate) inflight: usize,
    /// LRU bytes at that point.
    pub(crate) used: usize,
    /// Posted files at that point.
    pub(crate) files: usize,
    /// Decoded frames at that point.
    pub(crate) frames: usize,
}

/// One process-local cache of posted files and decoded frames.
pub struct Viewer {
    generation: u64,
    serial: AtomicU64,
    cache_bytes: usize,
    cache: CacheHandle,
    recycle_tx: Sender<Vec<u8>>,
    permits: PermitPool,
    /// Bytes of a body being read, or of a decode sample buffer not yet cached.
    inflight: Arc<AtomicUsize>,
    /// Every grey byte under the cap, shared with the decode workers.
    grey_gate: Arc<GreyGate>,
    connections: AtomicUsize,
    connection_cap: usize,
    /// Request bodies being read or still held by the request that read them.
    bodies_now: AtomicUsize,
    bodies_peak: AtomicUsize,
    /// Threads that have read a request body.
    body_threads: Mutex<HashSet<ThreadId>>,
    peak_used: AtomicUsize,
    peak_inflight: AtomicUsize,
    peak_workspace: AtomicUsize,
    peak_files: AtomicUsize,
    peak_frames: AtomicUsize,
    /// Set for the long-lived server. Tests keep the direct permit path.
    scheduler: Option<schedule::Scheduler>,
    #[cfg(test)]
    steps: Mutex<Vec<InflightStep>>,
    #[cfg(test)]
    preview_calls: AtomicU64,
}

impl Viewer {
    /// A viewer with the default connection cap of 32.
    ///
    /// Decodes and request bodies run on the caller. The process server uses
    /// [`Self::server`] so those allocations stay on two workers.
    #[must_use]
    pub fn new(config: ViewerConfig) -> Self {
        Self::with_connection_cap(config, CONNECTION_CAP)
    }

    /// The process server. Request bodies are allocated and freed on the `P`
    /// workers, and every decode runs there too, under a decode permit. The
    /// connection thread reads a body into the buffer a worker allocated.
    /// `P` is 2.
    ///
    /// At most four request bodies are live. That is the coefficient of `S_max`.
    #[must_use]
    pub fn server(config: ViewerConfig) -> Self {
        let mut viewer = Self::with_connection_cap(config, CONNECTION_CAP);
        viewer.scheduler = Some(schedule::Scheduler::start(Arc::clone(&viewer.grey_gate)));
        viewer
    }

    /// A viewer with an explicit connection cap. Zero refuses every connection.
    #[must_use]
    pub fn with_connection_cap(config: ViewerConfig, cap: usize) -> Self {
        let mut idle = Vec::with_capacity(PERMITS);
        for _ in 0..PERMITS {
            idle.push(Workspace::default());
        }
        let (recycle_tx, recycle_rx) = std::sync::mpsc::channel();
        Self {
            generation: mix_generation(),
            serial: AtomicU64::new(0),
            cache_bytes: config.cache_bytes,
            cache: CacheHandle {
                inner: Arc::new(Mutex::new(CacheState {
                    lru: Lru::new(CacheTier::Decoded, config.cache_bytes),
                    files: 0,
                    frames: 0,
                    free: HashMap::new(),
                    free_bytes: 0,
                    limit: config.cache_bytes,
                    frame_order: VecDeque::new(),
                    recycle_rx,
                    in_flight: BTreeMap::new(),
                    doomed: BTreeSet::new(),
                    #[cfg(test)]
                    reused: 0,
                    #[cfg(test)]
                    fresh: 0,
                })),
            },
            recycle_tx,
            permits: PermitPool {
                idle: Mutex::new(PoolState {
                    idle,
                    spare_grey: Vec::new(),
                }),
                ready: Condvar::new(),
                out: AtomicUsize::new(0),
            },
            inflight: Arc::new(AtomicUsize::new(0)),
            grey_gate: Arc::new(GreyGate::new(GREY_CAP)),
            connections: AtomicUsize::new(0),
            connection_cap: cap,
            bodies_now: AtomicUsize::new(0),
            bodies_peak: AtomicUsize::new(0),
            body_threads: Mutex::new(HashSet::new()),
            peak_used: AtomicUsize::new(0),
            peak_inflight: AtomicUsize::new(0),
            peak_workspace: AtomicUsize::new(0),
            peak_files: AtomicUsize::new(0),
            peak_frames: AtomicUsize::new(0),
            scheduler: None,
            #[cfg(test)]
            steps: Mutex::new(Vec::new()),
            #[cfg(test)]
            preview_calls: AtomicU64::new(0),
        }
    }

    /// The cache pressure reading. Not a use.
    #[must_use]
    pub fn pressure(&self) -> Pressure {
        self.census().0
    }

    pub(crate) fn census(&self) -> (Pressure, usize, usize) {
        let memory = self.memory_census();
        (memory.pressure, memory.files, memory.frames)
    }

    /// Cache bytes, in-flight body bytes and permit buffers.
    pub(crate) fn memory_census(&self) -> MemoryCensus {
        let mut state = self.lock_cache();
        state.drain_recycled();
        let mut pressure = state.lru.pressure();
        pressure.budget = self.cache_bytes;
        debug_assert!(
            state.charged() <= state.limit,
            "cached frames plus free buffers exceed the budget"
        );
        let files = state.files;
        let frames = state.frames;
        let free = state.free_bytes;
        drop(state);
        let permits = lock_mutex(&self.permits.idle);
        let mut workspace = self.permits.out.load(Ordering::Relaxed);
        for item in &permits.idle {
            workspace = workspace.saturating_add(workspace_bytes(item));
        }
        for grey in &permits.spare_grey {
            workspace = workspace.saturating_add(grey.capacity());
        }
        drop(permits);
        if let Some(scheduler) = &self.scheduler {
            workspace = workspace.saturating_add(scheduler.workspace_bytes());
        }
        let (grey_replies, grey_returned) = self.grey_gate.held();
        let census = MemoryCensus {
            pressure,
            files,
            frames,
            inflight: self.inflight.load(Ordering::Relaxed),
            grey: grey_replies.saturating_add(grey_returned),
            grey_replies,
            workspace,
            free,
        };
        bump_peak(&self.peak_used, census.pressure.used);
        bump_peak(&self.peak_inflight, census.inflight);
        bump_peak(&self.peak_workspace, census.workspace);
        bump_peak(&self.peak_files, census.files);
        bump_peak(&self.peak_frames, census.frames);
        census
    }

    /// Bodies in flight, threads that have read one, and the census peaks.
    pub(crate) fn body_schedule(&self) -> (usize, usize, usize) {
        if let Some(scheduler) = &self.scheduler {
            return scheduler.body_schedule();
        }
        let threads = lock_mutex(&self.body_threads).len();
        (
            threads,
            self.bodies_peak.load(Ordering::Relaxed),
            self.bodies_now.load(Ordering::Relaxed),
        )
    }

    /// Peak cache, inflight and workspace bytes, then peak file and frame counts.
    pub(crate) fn census_peaks(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.peak_used.load(Ordering::Relaxed),
            self.peak_inflight.load(Ordering::Relaxed),
            self.peak_workspace.load(Ordering::Relaxed),
            self.peak_files.load(Ordering::Relaxed),
            self.peak_frames.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn enter_body(&self) {
        let now = self
            .bodies_now
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        bump_peak(&self.bodies_peak, now);
        lock_mutex(&self.body_threads).insert(std::thread::current().id());
    }

    pub(crate) fn leave_body(&self) {
        let mut current = self.bodies_now.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(1);
            match self.bodies_now.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
        }
    }

    /// Record inflight, LRU bytes, files and frames.
    #[cfg(test)]
    pub(crate) fn trace_inflight(&self, label: &'static str) {
        let census = self.memory_census();
        lock_mutex(&self.steps).push(InflightStep {
            label,
            inflight: census.inflight,
            used: census.pressure.used,
            files: census.files,
            frames: census.frames,
        });
    }

    /// Snapshots from [`Self::trace_inflight`], in call order.
    #[cfg(test)]
    pub(crate) fn inflight_steps(&self) -> Vec<InflightStep> {
        lock_mutex(&self.steps).clone()
    }

    pub(crate) fn inflight_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.inflight)
    }

    /// The grey reply cap, shared with the decode jobs.
    pub(crate) fn grey_gate(&self) -> Arc<GreyGate> {
        Arc::clone(&self.grey_gate)
    }

    pub(crate) fn enter_inflight(&self, bytes: usize) {
        self.inflight.fetch_add(bytes, Ordering::Relaxed);
    }

    pub(crate) fn release_counter(counter: &AtomicUsize, bytes: usize) {
        let mut current = counter.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(bytes);
            match counter.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
        }
    }

    pub(crate) fn leave_inflight(&self, bytes: usize) {
        Self::release_counter(&self.inflight, bytes);
    }

    #[cfg(test)]
    pub(crate) fn preview_calls(&self) -> u64 {
        self.preview_calls.load(Ordering::Relaxed)
    }

    pub(crate) fn schedules(&self) -> bool {
        self.scheduler.is_some()
    }

    pub(crate) fn acquire_body(&self, live: &mut dyn FnMut() -> bool) -> Option<BodyPermit> {
        self.scheduler.as_ref()?.acquire_body(live)
    }

    pub(crate) fn acquire_decode(
        &self,
        home: Option<u8>,
        live: &mut dyn FnMut() -> bool,
    ) -> Option<DecodePermit> {
        self.scheduler.as_ref()?.acquire_decode(home, live)
    }

    #[cfg(test)]
    pub(crate) fn insert_file(&self, bytes: Vec<u8>) -> Option<FileId> {
        self.insert_posted(bytes, None, None)
    }

    pub(crate) fn insert_posted(
        &self,
        bytes: Vec<u8>,
        worker: Option<u8>,
        retire: Option<Sender<Vec<u8>>>,
    ) -> Option<FileId> {
        let id = self.allocate_id();
        let key = HeldKey::File(id);
        let posted = Arc::new(PostedBytes {
            bytes,
            worker,
            retire,
        });
        let cost = posted.capacity();
        let mut state = self.lock_cache();
        state.drain_recycled();
        state.reserve_for_insert(cost);
        let admission = state.lru.insert(key, Held::File(posted));
        let kept = admission.refused.is_none();
        state.absorb(key, 0, admission);
        drop(state);
        if kept { Some(id) } else { None }
    }

    #[cfg(test)]
    pub(crate) fn lookup_frame(&self, id: FileId, frame: u32) -> Lookup {
        let mut state = self.lock_cache();
        if let Some(Held::Frame(held)) = state.lru.get(&HeldKey::Frame(id, frame)) {
            return Lookup::Hit(Arc::clone(held));
        }
        if let Some(Held::File(bytes)) = state.lru.get(&HeldKey::File(id)) {
            return Lookup::Miss(Arc::clone(bytes));
        }
        Lookup::Gone
    }

    /// Frame or shared posted bytes. The file stays in the cache.
    pub(crate) fn preview_source(&self, id: FileId, frame: u32) -> PreviewSource {
        let mut state = self.lock_cache();
        let frame_key = HeldKey::Frame(id, frame);
        let found = if let Some(Held::Frame(held)) = state.lru.get(&frame_key) {
            let held = Arc::clone(held);
            state.touch_frame(frame_key);
            PreviewSource::Hit(held)
        } else {
            match state.lru.get(&HeldKey::File(id)) {
                Some(Held::File(bytes)) => {
                    let bytes = Arc::clone(bytes);
                    let flight = state.take_off(id, &self.cache.inner);
                    PreviewSource::Shared(SharedBytes {
                        bytes,
                        flight: Some(flight),
                    })
                }
                Some(Held::Frame(_)) | None => PreviewSource::Gone,
            }
        };
        drop(state);
        #[cfg(test)]
        self.preview_calls.fetch_add(1, Ordering::Relaxed);
        found
    }

    pub(crate) fn store_frame(&self, id: FileId, index: u32, held: HeldFrame) -> Arc<CachedFrame> {
        self.store_frame_with(id, index, held, None, None)
    }

    pub(crate) fn store_frame_with(
        &self,
        id: FileId,
        index: u32,
        held: HeldFrame,
        worker: Option<u8>,
        retire: Option<Sender<Vec<u8>>>,
    ) -> Arc<CachedFrame> {
        let frames = held.frames;
        let sample_len = held.samples.len();
        let charged = held.charged_bytes();
        let value = Arc::new(CachedFrame {
            inner: held,
            worker,
            retire,
            recycle: Some(self.recycle_tx.clone()),
        });
        let key = HeldKey::Frame(id, index);
        let mut state = self.lock_cache();
        if state.doomed.contains(&id) {
            // Forgotten while this decode ran. The frame answers its own
            // request and is not stored.
            drop(state);
            return value;
        }
        state.drain_recycled();
        state.reserve_for_insert(charged);
        let admission = state.lru.insert(key, Held::Frame(Arc::clone(&value)));
        let kept = admission.refused.is_none();
        state.absorb(key, sample_len, admission);
        let removed = if kept {
            take_encoded_if_ready(&mut state, id, frames)
        } else {
            None
        };
        drop(state);
        drop(removed);
        value
    }

    /// Put a sample buffer back on the free list when a decode does not store it.
    pub(crate) fn recycle_sample_buffer(&self, buffer: Vec<u8>) {
        self.cache.recycle_sample_buffer(buffer);
    }

    pub(crate) fn cache_handle(&self) -> CacheHandle {
        self.cache.clone()
    }

    /// Decodes that took a free buffer, then decodes that allocated.
    #[cfg(test)]
    pub(crate) fn buffer_claims(&self) -> (usize, usize) {
        let state = self.lock_cache();
        (state.reused, state.fresh)
    }

    /// Address of one free buffer of this exact length.
    #[cfg(test)]
    pub(crate) fn free_sample_ptr(&self, len: usize) -> Option<usize> {
        let mut state = self.lock_cache();
        state.drain_recycled();
        state.free.get(&len)?.last().map(|buf| buf.as_ptr().addr())
    }

    /// Overwrite every free buffer of this exact length with `byte`.
    ///
    /// A decode that read a recycled buffer before writing it would then
    /// show `byte` in its output.
    #[cfg(test)]
    pub(crate) fn poison_free_samples(&self, len: usize, byte: u8) -> usize {
        let mut state = self.lock_cache();
        state.drain_recycled();
        let Some(pile) = state.free.get_mut(&len) else {
            return 0;
        };
        for buffer in pile.iter_mut() {
            buffer.fill(byte);
        }
        pile.len()
    }

    /// Address and length of one cached frame's samples.
    #[cfg(test)]
    pub(crate) fn cached_sample(&self, id: FileId, frame: u32) -> Option<(usize, usize)> {
        let mut state = self.lock_cache();
        match state.lru.get(&HeldKey::Frame(id, frame))? {
            Held::Frame(arc) => Some((arc.samples.as_ptr().addr(), arc.samples.len())),
            Held::File(_) => None,
        }
    }

    /// Forget every id in `ids`: its posted bytes and every frame of it.
    ///
    /// Explicit release, not eviction. The frames leave through the
    /// bookkeeping eviction uses, `forget_frame` and `reclaim_held`, so the
    /// frame order, the counts and the free list are as if the id had never
    /// been held. An id that held nothing, another generation's id or a
    /// repeat, counts as unknown.
    pub(crate) fn forget(&self, ids: &[FileId]) -> Forgotten {
        let mut forgotten = Forgotten::default();
        let mut posted = Vec::new();
        let mut state = self.lock_cache();
        state.drain_recycled();
        for &id in ids {
            if state.in_flight.contains_key(&id) {
                state.doomed.insert(id);
            }
            let mut held = false;
            if let Some(file) = state.lru.remove(&HeldKey::File(id)) {
                held = true;
                forgotten.files = forgotten.files.saturating_add(1);
                forgotten.bytes = forgotten.bytes.saturating_add(file.bytes());
                state.files = state.files.saturating_sub(1);
                posted.push(file);
            }
            let frames = state
                .lru
                .remove_range(HeldKey::Frame(id, 0)..=HeldKey::Frame(id, u32::MAX));
            for (key, frame) in frames {
                held = true;
                forgotten.frames = forgotten.frames.saturating_add(1);
                forgotten.bytes = forgotten.bytes.saturating_add(frame.bytes());
                state.forget_frame(&key);
                state.frames = state.frames.saturating_sub(1);
                state.reclaim_held(frame);
            }
            if !held {
                forgotten.unknown = forgotten.unknown.saturating_add(1);
            }
        }
        drop(state);
        // Posted bytes go back to the worker that allocated them, outside the lock.
        drop(posted);
        forgotten
    }

    /// Stop the decode workers. Slots are still granted and jobs go unsent.
    #[cfg(test)]
    pub(crate) fn stop_decode_workers(&self) {
        if let Some(scheduler) = &self.scheduler {
            scheduler.stop_workers();
        }
    }

    /// Refuse every later decode and body slot.
    #[cfg(test)]
    pub(crate) fn shut_decode_workers(&self) {
        if let Some(scheduler) = &self.scheduler {
            scheduler.shut();
        }
    }

    /// Give a decode worker a returned grey buffer of `bytes`.
    #[cfg(test)]
    pub(crate) fn park_returned_grey(&self, worker: usize, bytes: usize) -> bool {
        self.scheduler
            .as_ref()
            .is_some_and(|scheduler| scheduler.park_grey(worker, bytes))
    }

    /// Move one cached frame's samples onto the free list.
    #[cfg(test)]
    pub(crate) fn evict_frame_to_free_list(&self, id: FileId, index: u32) -> bool {
        let mut state = self.lock_cache();
        let key = HeldKey::Frame(id, index);
        let Some(held) = state.lru.remove(&key) else {
            return false;
        };
        state.forget_frame(&key);
        state.frames = state.frames.saturating_sub(1);
        state.reclaim_held(held);
        true
    }

    pub(crate) fn file_bytes(&self, id: FileId) -> Option<Arc<PostedBytes>> {
        let mut state = self.lock_cache();
        match state.lru.get(&HeldKey::File(id))? {
            Held::File(bytes) => Some(Arc::clone(bytes)),
            Held::Frame(_) => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_workspace<T>(&self, body: impl FnOnce(&mut Workspace) -> T) -> T {
        let mut guard = loop {
            if let Some(guard) = self.poll_workspace(&mut || true) {
                break guard;
            }
        };
        body(&mut guard.workspace)
    }

    /// A permit, or `None` when `live` is false before one is free.
    ///
    /// The wait wakes every 50 ms so a caller whose client has gone does not
    /// sit on the condvar, and does not keep the connection slot.
    pub(crate) fn loan_workspace(
        &self,
        live: &mut dyn FnMut() -> bool,
    ) -> Option<WorkspaceGuard<'_>> {
        self.poll_workspace(live)
    }

    /// Put a grey reply buffer back on a permit. More than two are dropped.
    pub(crate) fn recycle_grey(&self, mut grey: Vec<u8>) {
        grey.clear();
        let mut state = lock_mutex(&self.permits.idle);
        if state.spare_grey.len() < PERMITS {
            state.spare_grey.push(grey);
        }
    }

    /// Native and grey capacity held by idle permits and returned grey buffers.
    #[cfg(test)]
    pub(crate) fn byte_buffer_capacities(&self) -> (usize, usize) {
        let state = lock_mutex(&self.permits.idle);
        let mut native = 0_usize;
        let mut grey = 0_usize;
        for workspace in &state.idle {
            native = native.saturating_add(workspace.native.capacity());
            grey = grey.saturating_add(workspace.grey.capacity());
        }
        for buffer in &state.spare_grey {
            grey = grey.saturating_add(buffer.capacity());
        }
        (native, grey)
    }

    pub(crate) fn acquire_connection(&self) -> bool {
        let mut current = self.connections.load(Ordering::Relaxed);
        loop {
            if current >= self.connection_cap {
                return false;
            }
            match self.connections.compare_exchange_weak(
                current,
                current.saturating_add(1),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }

    pub(crate) fn release_connection(&self) {
        self.connections.fetch_sub(1, Ordering::AcqRel);
    }

    fn allocate_id(&self) -> FileId {
        let serial = self.serial.fetch_add(1, Ordering::Relaxed);
        FileId {
            generation: self.generation,
            serial,
        }
    }

    fn lock_cache(&self) -> MutexGuard<'_, CacheState> {
        lock_mutex(&self.cache.inner)
    }

    fn poll_workspace(&self, live: &mut dyn FnMut() -> bool) -> Option<WorkspaceGuard<'_>> {
        let mut state = lock_mutex(&self.permits.idle);
        let wait = Duration::from_millis(50);
        while state.idle.is_empty() {
            if !live() {
                return None;
            }
            let (guard, _) = self
                .permits
                .ready
                .wait_timeout(state, wait)
                .unwrap_or_else(|poison| poison.into_inner());
            state = guard;
            if !live() {
                return None;
            }
        }
        let mut workspace = state.idle.pop().unwrap_or_default();
        if workspace.grey.capacity() == 0
            && let Some(grey) = state.spare_grey.pop()
        {
            workspace.grey = grey;
        }
        let charged = workspace_bytes(&workspace);
        self.permits.out.fetch_add(charged, Ordering::Relaxed);
        Some(WorkspaceGuard {
            pool: &self.permits,
            workspace,
            charged,
        })
    }
}

impl CacheHandle {
    /// A sample buffer of `len` bytes, from the free list or from an evicted frame.
    ///
    /// An empty buffer means the caller allocates. A returned buffer is owned
    /// by this decode and is not shared with the cache.
    pub(crate) fn claim_sample_buffer(&self, len: usize) -> Vec<u8> {
        lock_mutex(&self.inner).claim_buffer(len)
    }

    /// A parked free buffer of `len` bytes, for a decode that stores no frame.
    ///
    /// This is a pop from the free list and nothing else. It never steals a
    /// cached frame and never evicts, because the caller's frame is never
    /// stored and no room is needed for it. An empty buffer means the caller
    /// allocates.
    pub(crate) fn claim_parked_buffer(&self, len: usize) -> Vec<u8> {
        let mut state = lock_mutex(&self.inner);
        state.drain_recycled();
        state.pop_free(len).unwrap_or_default()
    }

    pub(crate) fn recycle_sample_buffer(&self, buffer: Vec<u8>) {
        if buffer.capacity() == 0 {
            return;
        }
        let mut state = lock_mutex(&self.inner);
        state.drain_recycled();
        state.park_if_fits(buffer);
    }
}

/// The charge a claim makes room for before the frame is decoded.
///
/// The sample bytes, the struct, the boxed value facts a grey frame carries
/// and the longest unit text a file can name, so the charge when the frame
/// is stored never exceeds this claim.
fn frame_sample_cost(len: usize) -> usize {
    len.saturating_add(size_of::<HeldFrame>())
        .saturating_add(size_of::<crate::value::ValueFacts>())
        .saturating_add(crate::value::UNIT_MAX_BYTES)
}

impl CacheState {
    /// Count one more decode holding `id`'s posted bytes.
    fn take_off(&mut self, id: FileId, cache: &Arc<Mutex<CacheState>>) -> Flight {
        let count = self.in_flight.entry(id).or_insert(0);
        *count = count.saturating_add(1);
        Flight {
            cache: Arc::clone(cache),
            id,
        }
    }

    /// One decode of `id` is done. The last one takes `id` out of both sets.
    fn land(&mut self, id: FileId) {
        let Some(count) = self.in_flight.get_mut(&id) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 {
            self.in_flight.remove(&id);
            self.doomed.remove(&id);
        }
    }

    fn drain_recycled(&mut self) {
        while let Ok(buffer) = self.recycle_rx.try_recv() {
            self.park_if_fits(buffer);
        }
    }

    fn charged(&self) -> usize {
        self.lru.pressure().used.saturating_add(self.free_bytes)
    }

    /// Claim a sample buffer of `len` bytes for a decode.
    ///
    /// Every route makes room for the whole `frame_sample_cost` of the buffer
    /// it returns: a parked buffer, a buffer stolen from the oldest frame of
    /// the same length when no reply reads it, and a fresh one. A stolen
    /// frame may have charged less than the new frame will, so its freed
    /// bytes alone are not the room.
    fn claim_buffer(&mut self, len: usize) -> Vec<u8> {
        self.drain_recycled();
        if let Some(buffer) = self.pop_free(len) {
            return self.reuse_claimed(buffer);
        }
        let cost = frame_sample_cost(len);
        if self.charged().saturating_add(cost) <= self.limit {
            self.note_fresh();
            return Vec::new();
        }
        if let Some(buffer) = self.take_oldest_frame_buffer(len) {
            self.park_if_fits(buffer);
            if let Some(buffer) = self.pop_free(len) {
                return self.reuse_claimed(buffer);
            }
        }
        self.make_room_dropping(cost);
        self.note_fresh();
        Vec::new()
    }

    fn reuse_claimed(&mut self, buffer: Vec<u8>) -> Vec<u8> {
        self.note_reused();
        self.make_room_dropping(frame_sample_cost(buffer.capacity()));
        buffer
    }

    fn note_reused(&mut self) {
        #[cfg(test)]
        {
            self.reused = self.reused.saturating_add(1);
        }
    }

    fn note_fresh(&mut self) {
        #[cfg(test)]
        {
            self.fresh = self.fresh.saturating_add(1);
        }
    }

    fn pop_free(&mut self, len: usize) -> Option<Vec<u8>> {
        let pile = self.free.get_mut(&len)?;
        let buffer = pile.pop()?;
        if pile.is_empty() {
            self.free.remove(&len);
        }
        self.free_bytes = self.free_bytes.saturating_sub(buffer.capacity());
        Some(buffer)
    }

    fn push_free(&mut self, buffer: Vec<u8>) {
        let len = buffer.len();
        let cost = buffer.capacity();
        if len == 0 || cost == 0 {
            return;
        }
        self.free_bytes = self.free_bytes.saturating_add(cost);
        self.free.entry(len).or_default().push(buffer);
    }

    /// Keep the buffer only when cached bytes plus free bytes still fit.
    fn park_if_fits(&mut self, buffer: Vec<u8>) {
        let cost = buffer.capacity();
        if buffer.is_empty() || cost == 0 {
            return;
        }
        if self.charged().saturating_add(cost) > self.limit {
            return;
        }
        self.push_free(buffer);
    }

    /// Drop free buffers, then cached entries, until `cost` more bytes fit.
    ///
    /// A cached frame is discarded here. Its sample buffer is not parked,
    /// because this room is for a buffer the caller already holds. A frame a
    /// reply is reading is passed over, because evicting it frees no memory
    /// and makes that reply's next request answer 410.
    fn make_room_dropping(&mut self, cost: usize) {
        loop {
            if self.charged().saturating_add(cost) <= self.limit {
                return;
            }
            if self.drop_one_free() {
                continue;
            }
            let evicted = self.evict_oldest_unread();
            if evicted.is_empty() {
                return;
            }
            for held in evicted {
                self.discard_held(held);
            }
        }
    }

    /// Make room for a new cache entry.
    ///
    /// An entry larger than the whole budget is left for `Lru` to refuse, so
    /// a refusal does not empty the cache. A frame that leaves is parked when
    /// the incoming entry and that buffer both fit, and dropped when they do not.
    fn reserve_for_insert(&mut self, cost: usize) {
        if cost > self.limit {
            return;
        }
        loop {
            if self.charged().saturating_add(cost) <= self.limit {
                return;
            }
            if self.drop_one_free() {
                continue;
            }
            let evicted = self.evict_oldest();
            if evicted.is_empty() {
                return;
            }
            for held in evicted {
                self.keep_buffer_if_room(held, cost);
            }
        }
    }

    fn drop_one_free(&mut self) -> bool {
        let Some(&len) = self.free.keys().next() else {
            return false;
        };
        let Some(buffer) = self.free.get_mut(&len).and_then(Vec::pop) else {
            self.free.remove(&len);
            return false;
        };
        if self.free.get(&len).is_some_and(Vec::is_empty) {
            self.free.remove(&len);
        }
        self.free_bytes = self.free_bytes.saturating_sub(buffer.capacity());
        true
    }

    fn keep_buffer_if_room(&mut self, held: Held, incoming: usize) {
        let Held::Frame(arc) = held else {
            self.discard_held(held);
            return;
        };
        let cap = arc.samples.capacity();
        if Arc::strong_count(&arc) == 1
            && self.charged().saturating_add(cap).saturating_add(incoming) <= self.limit
        {
            self.reclaim_held(Held::Frame(arc));
            return;
        }
        self.discard_held(Held::Frame(arc));
    }

    /// Evict the least recently used entry that is not a frame being read.
    ///
    /// A frame a reply holds moves to the most recent end and stays. Each
    /// entry is looked at once at most, so a cache of frames all being read
    /// gives up with nothing evicted.
    fn evict_oldest_unread(&mut self) -> Vec<Held> {
        for _ in 0..self.lru.len() {
            let mut evicted = Vec::new();
            for (key, held) in self.evict_oldest_keyed() {
                let reading = matches!(&held, Held::Frame(arc) if Arc::strong_count(arc) > 1);
                if !reading {
                    self.forget_evicted(&key);
                    evicted.push(held);
                    continue;
                }
                let admission = self.lru.insert(key, held);
                self.touch_frame(key);
                for (other, dropped) in admission.evicted {
                    self.forget_evicted(&other);
                    evicted.push(dropped);
                }
                if let Some((refused, dropped)) = admission.refused {
                    self.forget_evicted(&refused);
                    evicted.push(dropped);
                }
            }
            if !evicted.is_empty() {
                return evicted;
            }
        }
        Vec::new()
    }

    fn forget_evicted(&mut self, key: &HeldKey) {
        self.forget_frame(key);
        match key {
            HeldKey::File(_) => self.files = self.files.saturating_sub(1),
            HeldKey::Frame(_, _) => self.frames = self.frames.saturating_sub(1),
        }
    }

    /// Evict one least recently used entry. The caller parks or drops it.
    fn evict_oldest(&mut self) -> Vec<Held> {
        let evicted = self.evict_oldest_keyed();
        let mut out = Vec::new();
        for (key, held) in evicted {
            self.forget_evicted(&key);
            out.push(held);
        }
        out
    }

    /// Take the least recently used entry out of the LRU, keys and all,
    /// without touching the counts.
    fn evict_oldest_keyed(&mut self) -> Vec<(HeldKey, Held)> {
        if self.lru.is_empty() {
            return Vec::new();
        }
        let budget = self.lru.budget();
        let used = self.lru.pressure().used;
        let before_len = self.lru.len();
        let target = used.saturating_sub(1);
        let admission = self.lru.set_budget(target);
        let _ = self.lru.set_budget(budget);
        if self.lru.len() == before_len {
            return Vec::new();
        }
        admission.evicted
    }

    fn take_oldest_frame_buffer(&mut self, len: usize) -> Option<Vec<u8>> {
        let position = self.frame_order.iter().position(|frame| frame.len == len)?;
        let tracked = self.frame_order.remove(position)?;
        let key = tracked.key;
        let held = self.lru.remove(&key)?;
        let Held::Frame(arc) = held else {
            let _ = self.lru.insert(key, held);
            self.frame_order.push_back(tracked);
            return None;
        };
        if Arc::strong_count(&arc) > 1 {
            self.frame_order.push_back(tracked);
            let admission = self.lru.insert(key, Held::Frame(arc));
            for (evicted_key, evicted) in admission.evicted {
                self.forget_evicted(&evicted_key);
                self.discard_held(evicted);
            }
            if let Some((_, refused)) = admission.refused {
                self.frames = self.frames.saturating_sub(1);
                self.forget_frame(&key);
                self.discard_held(refused);
            }
            return None;
        }
        self.frames = self.frames.saturating_sub(1);
        reclaim_owned(arc)
    }

    fn absorb(&mut self, key: HeldKey, sample_len: usize, admission: Admission<HeldKey, Held>) {
        if admission.refused.is_some() {
            if let Some((_, held)) = admission.refused {
                self.reclaim_held(held);
            }
            return;
        }
        let displaced = admission.displaced.is_some();
        for (evicted_key, held) in admission.evicted {
            self.forget_evicted(&evicted_key);
            self.reclaim_held(held);
        }
        if let Some(held) = admission.displaced {
            self.reclaim_held(held);
        }
        if !displaced {
            match key {
                HeldKey::File(_) => self.files = self.files.saturating_add(1),
                HeldKey::Frame(_, _) => self.frames = self.frames.saturating_add(1),
            }
        }
        if matches!(key, HeldKey::Frame(_, _)) {
            self.remember_frame(key, sample_len);
        }
    }

    fn reclaim_held(&mut self, held: Held) {
        if let Held::Frame(arc) = held
            && let Some(buffer) = reclaim_owned(arc)
        {
            self.park_if_fits(buffer);
        }
    }

    fn discard_held(&mut self, held: Held) {
        if let Held::Frame(arc) = held
            && let Ok(mut frame) = Arc::try_unwrap(arc)
        {
            frame.recycle.take();
            frame.retire.take();
        }
    }

    fn remember_frame(&mut self, key: HeldKey, len: usize) {
        self.forget_frame(&key);
        self.frame_order.push_back(TrackedFrame { key, len });
    }

    fn touch_frame(&mut self, key: HeldKey) {
        if let Some(position) = self.frame_order.iter().position(|frame| frame.key == key)
            && let Some(frame) = self.frame_order.remove(position)
        {
            self.frame_order.push_back(frame);
        }
    }

    fn forget_frame(&mut self, key: &HeldKey) {
        if let Some(position) = self.frame_order.iter().position(|frame| frame.key == *key) {
            self.frame_order.remove(position);
        }
    }
}

fn reclaim_owned(arc: Arc<CachedFrame>) -> Option<Vec<u8>> {
    let mut frame = Arc::try_unwrap(arc).ok()?;
    let samples = std::mem::take(&mut frame.inner.samples);
    frame.recycle.take();
    frame.retire.take();
    Some(samples)
}

fn take_encoded_if_ready(state: &mut CacheState, id: FileId, frames: u32) -> Option<Held> {
    let complete = (0..frames).all(|index| state.lru.contains_key(&HeldKey::Frame(id, index)));
    if !complete {
        return None;
    }
    let removed = state.lru.remove(&HeldKey::File(id))?;
    state.files = state.files.saturating_sub(1);
    Some(removed)
}

fn mix_generation() -> u64 {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let tick = TICK.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let low = u64::try_from(nanos & u128::from(u64::MAX)).unwrap_or(0);
    let high = u64::try_from(nanos >> 64).unwrap_or(0);
    let rotate = u32::try_from(tick & 63).unwrap_or(0);
    low.rotate_left(rotate) ^ high ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::PreviewError;
    use ocelli_pixel::{
        ImageDimensions, PhotometricInterpretation, PixelRepresentation, SampleLayout, StoredBits,
        StoredPixelDescription,
    };

    fn minimal_frame(samples: Vec<u8>, rows: u32, columns: u32) -> Result<HeldFrame, String> {
        let dimensions = ImageDimensions::new(rows, columns)
            .map_err(|_| "dimensions were refused".to_owned())?;
        let bits = StoredBits::new(16, 16, 15, PixelRepresentation::Unsigned)
            .map_err(|_| "bits were refused".to_owned())?;
        let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)
            .map_err(|_| "layout was refused".to_owned())?;
        Ok(HeldFrame {
            samples,
            description: StoredPixelDescription::new(dimensions, layout, bits),
            byte_order: ocelli_pixel::ByteOrder::LittleEndian,
            recipe: Err(PreviewError::Window),
            facts: Err(PreviewError::Window),
            full_range: Err(PreviewError::Window),
            value: Err(PreviewError::Window),
            calibration: Err(PreviewError::Window),
            frames: 1,
            width: columns,
            height: rows,
        })
    }

    #[test]
    fn file_id_rejects_a_sign_and_a_leading_zero() {
        assert!(FileId::parse("a.b").is_some());
        assert!(FileId::parse("0.0").is_some());
        assert!(FileId::parse("+a.b").is_none());
        assert!(FileId::parse("a.+b").is_none());
        assert!(FileId::parse("0a.b").is_none());
        assert!(FileId::parse("a.00").is_none());
        assert!(FileId::parse("-1.2").is_none());
    }

    #[test]
    fn a_gone_caller_does_not_wait_for_a_permit() {
        let viewer = Viewer::new(ViewerConfig { cache_bytes: 1024 });
        let first = viewer.loan_workspace(&mut || true);
        let second = viewer.loan_workspace(&mut || true);
        assert!(first.is_some());
        assert!(second.is_some());
        let started = std::time::Instant::now();
        let missed = viewer.loan_workspace(&mut || false);
        assert!(missed.is_none());
        assert!(started.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn held_bytes_are_the_buffer_capacity() -> Result<(), String> {
        // 512 * 512 * 2 = 524288. One 16-bit sample per pixel, PS3.3 C.7.6.3.
        let sample_bytes = 512_usize * 512 * 2;
        let mut samples = vec![0_u8; sample_bytes];
        samples.shrink_to_fit();
        if samples.capacity() != sample_bytes {
            return Err(format!(
                "the sample buffer capacity was {}",
                samples.capacity()
            ));
        }
        let capacity = samples.capacity();
        let held = Held::Frame(Arc::new(CachedFrame::plain(minimal_frame(
            samples, 512, 512,
        )?)));
        let expected = capacity + size_of::<HeldFrame>();
        if held.bytes() != expected {
            return Err(format!(
                "held bytes were {} and the capacity sum is {expected}",
                held.bytes()
            ));
        }
        Ok(())
    }

    #[test]
    fn held_bytes_charge_the_value_facts() -> Result<(), String> {
        // Synthetic slice 0 is a CT with no Rescale Type, so it holds value
        // facts with the unit text HU. The charge is every owned buffer's
        // capacity plus the struct: the samples, the boxed facts and the
        // unit text, as F-216 defines `bytes()`.
        let bytes = crate::slice(0).ok_or("no synthetic slice")?;
        let mut native = Vec::new();
        let mut samples = Vec::new();
        let frame = crate::preview::decode_held(
            &bytes,
            0,
            &mut native,
            &mut samples,
            &mut |_| {},
            &mut |_| Vec::new(),
        )
        .map_err(|error| format!("slice 0 was refused: {}", error.code()))?;
        let facts = frame
            .value
            .as_ref()
            .map_err(|error| format!("slice 0 has no value facts: {}", error.code()))?;
        let expected = frame.samples.capacity()
            + size_of::<HeldFrame>()
            + size_of::<crate::value::ValueFacts>()
            + facts.unit_capacity();
        let held = Held::Frame(Arc::new(CachedFrame::plain(frame)));
        if held.bytes() != expected {
            return Err(format!(
                "held bytes were {} and the owned sum is {expected}",
                held.bytes()
            ));
        }
        Ok(())
    }

    #[test]
    fn a_file_is_accounted_by_capacity() {
        let mut spare = Vec::with_capacity(64);
        spare.push(1_u8);
        let held = Held::File(Arc::new(PostedBytes::plain(spare)));
        assert!(held.bytes() >= 64);
        assert_ne!(held.bytes(), 1);
    }

    #[test]
    fn a_single_frame_object_keeps_only_the_decoded_frame() -> Result<(), String> {
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: 1024 * 1024,
        });
        let mut encoded = vec![7_u8; 4096];
        encoded.shrink_to_fit();
        let encoded_cost = encoded.capacity();
        let Some(id) = viewer.insert_file(encoded) else {
            return Err("the encoded bytes were refused".to_owned());
        };
        let (pressure, files, frames) = viewer.census();
        if files != 1 || frames != 0 || pressure.used != encoded_cost {
            return Err(format!(
                "before the frame, used {} files {files} frames {frames}",
                pressure.used
            ));
        }
        let mut samples = vec![0_u8; 2];
        samples.shrink_to_fit();
        let sample_cost = samples.capacity();
        let frame = minimal_frame(samples, 1, 1)?;
        let decoded_cost = sample_cost + size_of::<HeldFrame>();
        viewer.store_frame(id, 0, frame);
        let (pressure, files, frames) = viewer.census();
        if files != 0 || frames != 1 || pressure.used != decoded_cost {
            return Err(format!(
                "after the first frame, used {} files {files} frames {frames}, decoded {decoded_cost}",
                pressure.used
            ));
        }
        if pressure.used == encoded_cost || viewer.file_bytes(id).is_some() {
            return Err("the encoded bytes were still held".to_owned());
        }
        match viewer.lookup_frame(id, 0) {
            Lookup::Hit(frame) if frame.samples.len() == 2 => {}
            Lookup::Hit(frame) => {
                return Err(format!(
                    "the decoded frame held {} bytes",
                    frame.samples.len()
                ));
            }
            Lookup::Miss(bytes) => {
                return Err(format!("lookup returned the file, {} bytes", bytes.len()));
            }
            Lookup::Gone => {
                return Err("the decoded frame was not what lookup returned".to_owned());
            }
        }
        Ok(())
    }

    /// Wait until `preview_source` has been called `count` times.
    fn wait_for_preview_calls(viewer: &Viewer, count: u64) -> Result<(), String> {
        let started = std::time::Instant::now();
        while viewer.preview_calls() < count {
            if started.elapsed() > Duration::from_secs(5) {
                return Err("the decode did not reach the posted bytes".to_owned());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    #[test]
    fn a_forget_during_a_decode_is_not_undone_by_its_store() -> Result<(), String> {
        // F-226 AC12. Synthetic slice 0, 128 columns by 96 rows, is posted.
        // Both permits are held inside `with_workspace` until the test
        // releases them. A decode takes the posted bytes through
        // `preview_source` and waits for a permit, as `/preview` does on the
        // direct path. The id is forgotten while it waits, then the permits
        // are released. The decode still returns its frame, so its reply is
        // served, and its store inserts nothing. It keeps its flight until
        // after the store, as every route does, and the last flight out
        // leaves no id in either set.
        let viewer = Arc::new(Viewer::new(ViewerConfig {
            cache_bytes: 8 * 1024 * 1024,
        }));
        let bytes = crate::slice(0).ok_or("no synthetic slice")?;
        let posted = bytes.len();
        let id = viewer
            .insert_file(bytes)
            .ok_or("the synthetic slice was not held")?;
        let entered = Arc::new(std::sync::Barrier::new(PERMITS + 1));
        let release = Arc::new(std::sync::Barrier::new(PERMITS + 1));
        let mut holders = Vec::new();
        for _ in 0..PERMITS {
            let viewer = Arc::clone(&viewer);
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            holders.push(std::thread::spawn(move || {
                viewer.with_workspace(|_| {
                    entered.wait();
                    release.wait();
                });
            }));
        }
        entered.wait();
        let decoding = Arc::clone(&viewer);
        let decode = std::thread::spawn(move || -> Result<(u32, u32), String> {
            let PreviewSource::Shared(mut shared) = decoding.preview_source(id, 0) else {
                return Err("the posted bytes were not handed to the decode".to_owned());
            };
            let flight = shared.take_flight();
            let held = decoding
                .with_workspace(|workspace| {
                    let mut samples = Vec::new();
                    crate::preview::decode_held(
                        shared.as_slice(),
                        0,
                        &mut workspace.native,
                        &mut samples,
                        &mut |_| {},
                        &mut |_| Vec::new(),
                    )
                })
                .map_err(|error| format!("slice 0 was refused: {}", error.code()))?;
            drop(shared);
            let frame = decoding.store_frame(id, 0, held);
            drop(flight);
            Ok((frame.width, frame.height))
        });
        wait_for_preview_calls(&viewer, 1)?;
        let forgotten = viewer.forget(&[id]);
        let expected = Forgotten {
            files: 1,
            frames: 0,
            bytes: posted,
            unknown: 0,
        };
        release.wait();
        for holder in holders {
            holder
                .join()
                .map_err(|_| "a permit holder panicked".to_owned())?;
        }
        let served = decode
            .join()
            .map_err(|_| "the decode panicked".to_owned())??;
        if forgotten != expected {
            return Err(format!(
                "the forget removed {forgotten:?}, not {expected:?}"
            ));
        }
        if served != (128, 96) {
            return Err(format!("the decode returned a {served:?} frame"));
        }
        let (pressure, files, frames) = viewer.census();
        if files != 0 || frames != 0 || pressure.used != 0 {
            return Err(format!(
                "after the decode the cache held {files} files, {frames} frames, {} bytes",
                pressure.used
            ));
        }
        let state = viewer.lock_cache();
        if !state.doomed.is_empty() || !state.in_flight.is_empty() {
            return Err(format!(
                "{} doomed and {} in flight after the decode",
                state.doomed.len(),
                state.in_flight.len()
            ));
        }
        Ok(())
    }

    /// Frame serials held, in the frame order, oldest first.
    fn frame_serials(viewer: &Viewer) -> Vec<u64> {
        let state = viewer.lock_cache();
        state
            .frame_order
            .iter()
            .filter_map(|frame| match frame.key {
                HeldKey::Frame(id, _) => Some(id.serial),
                HeldKey::File(_) => None,
            })
            .collect()
    }

    #[test]
    fn a_forgotten_file_leaves_the_cache_as_if_never_held() -> Result<(), String> {
        // F-226, the lead's amendment. On `forgot`, file 9 is posted and its
        // frame 0 of two is stored, so the bytes and one frame are held, and
        // then 9 is forgotten. `fresh` never holds 9. Both then store frames
        // 1 to 4 of 64 sample bytes into a budget of three such frames, so
        // the fourth store evicts frame 1. The two must agree on files,
        // frames, used bytes, the frame order and which frames survive.
        let charge = Held::Frame(Arc::new(CachedFrame::plain(minimal_frame(
            vec![0_u8; 64],
            1,
            32,
        )?)))
        .bytes();
        let budget = charge.saturating_mul(3);
        let forgot = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let fresh = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let posted = forgot
            .insert_file(vec![7_u8; 32])
            .ok_or("the posted bytes were refused")?;
        let mut first = minimal_frame(vec![0_u8; 64], 1, 32)?;
        first.frames = 2;
        drop(forgot.store_frame(posted, 0, first));
        if forgot.census().1 != 1 || forgot.census().2 != 1 {
            return Err("the bytes and frame 0 were not both held".to_owned());
        }
        let gone = forgot.forget(&[posted]);
        if gone.files != 1 || gone.frames != 1 || gone.unknown != 0 {
            return Err(format!("the forget removed {gone:?}"));
        }
        if !frame_serials(&forgot).is_empty() {
            return Err("the frame order kept the forgotten frame".to_owned());
        }
        for viewer in [&forgot, &fresh] {
            for serial in 1..=4 {
                let id = FileId {
                    generation: viewer.generation,
                    serial,
                };
                drop(viewer.store_frame(id, 0, minimal_frame(vec![0_u8; 64], 1, 32)?));
            }
        }
        let (forgot_pressure, forgot_files, forgot_frames) = forgot.census();
        let (fresh_pressure, fresh_files, fresh_frames) = fresh.census();
        if (forgot_files, forgot_frames, forgot_pressure.used)
            != (fresh_files, fresh_frames, fresh_pressure.used)
        {
            return Err(format!(
                "forgot held {forgot_files} files {forgot_frames} frames {} bytes, fresh {fresh_files} {fresh_frames} {}",
                forgot_pressure.used, fresh_pressure.used
            ));
        }
        let order = frame_serials(&forgot);
        if order != frame_serials(&fresh) || order != [2, 3, 4] {
            return Err(format!(
                "the frame order was {order:?} after the forget and {:?} without it",
                frame_serials(&fresh)
            ));
        }
        for viewer in [&forgot, &fresh] {
            let first = FileId {
                generation: viewer.generation,
                serial: 1,
            };
            if !matches!(viewer.lookup_frame(first, 0), Lookup::Gone) {
                return Err("the oldest frame was not the one evicted".to_owned());
            }
        }
        Ok(())
    }

    #[test]
    fn held_keys_order_file_before_frame() {
        let low = FileId {
            generation: 1,
            serial: 1,
        };
        let high = FileId {
            generation: 1,
            serial: 2,
        };
        assert!(HeldKey::File(low) < HeldKey::File(high));
        assert!(HeldKey::File(high) < HeldKey::Frame(low, 0));
        assert!(HeldKey::Frame(low, 0) < HeldKey::Frame(low, 1));
        assert!(HeldKey::Frame(low, 1) < HeldKey::Frame(high, 0));
    }

    #[test]
    fn two_viewers_do_not_share_a_generation() -> Result<(), String> {
        let left = Viewer::new(ViewerConfig { cache_bytes: 0 });
        let right = Viewer::new(ViewerConfig { cache_bytes: 0 });
        if left.generation == right.generation {
            return Err("two viewers shared a generation".to_owned());
        }
        Ok(())
    }

    #[test]
    fn a_permit_keeps_the_scratch() -> Result<(), String> {
        let viewer = Viewer::new(ViewerConfig { cache_bytes: 0 });
        viewer.with_workspace(|workspace| workspace.scratch.resize(4, Stored(0.0)));
        viewer.with_workspace(|workspace| {
            if workspace.scratch.len() != 4 {
                return Err(format!(
                    "the scratch length was {}",
                    workspace.scratch.len()
                ));
            }
            Ok(())
        })
    }

    #[test]
    fn the_held_bytes_never_exceed_the_budget() -> Result<(), String> {
        let samples = vec![0_u8; 32];
        let frame = minimal_frame(samples, 1, 1)?;
        let cost = Held::Frame(Arc::new(CachedFrame::plain(minimal_frame(
            vec![0_u8; 32],
            1,
            1,
        )?)))
        .bytes();
        if cost == 0 {
            return Err("a held frame cost nothing".to_owned());
        }
        let viewer = Viewer::new(ViewerConfig { cache_bytes: cost });
        let first = FileId {
            generation: viewer.generation,
            serial: 1,
        };
        let second = FileId {
            generation: viewer.generation,
            serial: 2,
        };
        viewer.store_frame(first, 0, frame);
        let (pressure, _, frames) = viewer.census();
        if pressure.used != cost || frames != 1 {
            return Err(format!(
                "the first frame used {} across {frames} frames",
                pressure.used
            ));
        }
        viewer.store_frame(second, 0, minimal_frame(vec![0_u8; 32], 1, 1)?);
        let census = viewer.memory_census();
        if census.pressure.used != cost
            || census.pressure.used > census.pressure.budget
            || census.frames != 1
            || census.free != 0
            || census.pressure.used.saturating_add(census.free) > census.pressure.budget
        {
            return Err(format!(
                "after eviction used {} free {} budget {} frames {}",
                census.pressure.used, census.free, census.pressure.budget, census.frames
            ));
        }
        let mut too_big = Vec::with_capacity(cost + 1);
        too_big.push(0_u8);
        if viewer.insert_file(too_big).is_some() {
            return Err("a file larger than the budget was held".to_owned());
        }
        let (pressure, files, _) = viewer.census();
        if pressure.used != cost || files != 0 {
            return Err(format!(
                "a refused file changed used to {} and files to {files}",
                pressure.used
            ));
        }
        Ok(())
    }

    #[test]
    fn a_claim_passes_over_every_frame_being_read() -> Result<(), String> {
        // R1 and R2 are frames of 64 sample bytes that replies hold, at the
        // oldest end. U is an unread frame of 64 behind them. A claim for 4
        // bytes matches no cached length, so it steals nothing and must make
        // room for frame_sample_cost(4). The budget is the two readers plus
        // that cost, so evicting U alone is enough, and the claim has to look
        // past both readers to find it. The 4-byte frame then stores within
        // the budget and both readers stay held.
        let reader_charge = Held::Frame(Arc::new(CachedFrame::plain(minimal_frame(
            vec![0_u8; 64],
            1,
            32,
        )?)))
        .bytes();
        let budget = reader_charge
            .saturating_mul(2)
            .saturating_add(frame_sample_cost(4));
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let id = |serial: u64| FileId {
            generation: viewer.generation,
            serial,
        };
        let first = viewer.store_frame(id(1), 0, minimal_frame(vec![0_u8; 64], 1, 32)?);
        let second = viewer.store_frame(id(2), 0, minimal_frame(vec![0_u8; 64], 1, 32)?);
        drop(viewer.store_frame(id(3), 0, minimal_frame(vec![0_u8; 64], 1, 32)?));
        let mut samples = viewer.cache_handle().claim_sample_buffer(4);
        samples.clear();
        samples.resize(4, 0);
        drop(viewer.store_frame(id(4), 0, minimal_frame(samples, 1, 2)?));
        for (name, serial) in [("R1", 1), ("R2", 2)] {
            if !matches!(viewer.lookup_frame(id(serial), 0), Lookup::Hit(_)) {
                return Err(format!("{name} was evicted while a reply read it"));
            }
        }
        if matches!(viewer.lookup_frame(id(3), 0), Lookup::Hit(_)) {
            return Err("the unread frame U was kept".to_owned());
        }
        let census = viewer.memory_census();
        if census.pressure.used.saturating_add(census.free) > budget {
            return Err(format!(
                "used {} free {} budget {budget}",
                census.pressure.used, census.free
            ));
        }
        drop((first, second));
        Ok(())
    }

    #[test]
    fn cached_frames_and_free_buffers_share_one_budget() -> Result<(), String> {
        // Three frames of 64 bytes and three of 32 fit, with a little spare.
        let small = Held::Frame(Arc::new(CachedFrame::plain(minimal_frame(
            vec![0_u8; 32],
            1,
            1,
        )?)))
        .bytes();
        let large = small + 32;
        let budget = 3 * large + 3 * small + 8;
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let cache = viewer.cache_handle();
        let mut held = Vec::new();
        for step in 0_u64..200 {
            let len = if step % 3 == 0 { 64 } else { 32 };
            let mut samples = cache.claim_sample_buffer(len);
            if samples.capacity() < len {
                samples = Vec::with_capacity(len);
            }
            samples.clear();
            samples.resize(len, 0);
            let id = FileId {
                generation: viewer.generation,
                serial: step,
            };
            if step % 7 == 0 {
                // A decode that fails hands its buffer back.
                viewer.recycle_sample_buffer(samples);
            } else {
                let kept = viewer.store_frame(id, 0, minimal_frame(samples, 1, 1)?);
                // A reply still reading a frame keeps it past its eviction.
                if step % 5 == 0 {
                    held.push(kept);
                }
                if held.len() > 2 {
                    held.remove(0);
                }
            }
            let census = viewer.memory_census();
            if census.pressure.used.saturating_add(census.free) > budget {
                return Err(format!(
                    "step {step} used {} free {} budget {budget}",
                    census.pressure.used, census.free
                ));
            }
        }
        drop(held);
        let census = viewer.memory_census();
        if census.pressure.used.saturating_add(census.free) > budget {
            return Err(format!(
                "after the last reply used {} free {} budget {budget}",
                census.pressure.used, census.free
            ));
        }
        let (reused, fresh) = viewer.buffer_claims();
        if reused == 0 || reused < fresh {
            return Err(format!("reused {reused} fresh {fresh}"));
        }
        Ok(())
    }

    #[test]
    fn cache_mib_rejects_a_fraction() -> Result<(), String> {
        let error = match cache_mib_to_bytes("1.5") {
            Ok(_) => return Err("a fraction was accepted".to_owned()),
            Err(error) => error,
        };
        if error.kind() != std::io::ErrorKind::InvalidInput {
            return Err("a fraction was not invalid input".to_owned());
        }
        if error.to_string() != "OCELLI_CACHE_MIB is not a whole number" {
            return Err(error.to_string());
        }
        if cache_mib_to_bytes("0").map_err(|error| error.to_string())? != 0 {
            return Err("zero was not zero bytes".to_owned());
        }
        if cache_mib_to_bytes("256").map_err(|error| error.to_string())? != DEFAULT_CACHE_BYTES {
            return Err("256 MiB was not the default byte count".to_owned());
        }
        Ok(())
    }

    fn small_gate() -> Arc<GreyGate> {
        Arc::new(GreyGate::new(100))
    }

    #[test]
    fn a_refused_admission_releases_the_room_it_was_given() {
        let gate = small_gate();
        let other = gate.reserve(80);
        assert!(other.is_some());
        // 80 held elsewhere, 10 waited for, and the render needs 50.
        let waited = gate.reserve(10);
        assert!(waited.is_some());
        // The refusal hands back a place in the queue for the grey.
        let place = gate.admit(waited, 50, 0).err();
        assert_eq!(gate.held(), (80, 0));
        assert_eq!(place.as_ref().map(GreyWaiter::grey_len), Some(50));
        assert_eq!(gate.waiting(), 1);
        drop(place);
        assert_eq!(gate.waiting(), 0);
        // A grey larger than the cap is not admitted beside other grey.
        let waited = gate.reserve(10);
        assert!(gate.admit(waited, 101, 0).is_err());
        assert_eq!(gate.held(), (80, 0));
        drop(other);
        assert_eq!(gate.held(), (0, 0));
    }

    #[test]
    fn an_admission_moves_the_returned_buffer_into_the_reply() {
        let gate = small_gate();
        assert!(gate.count_returned(30));
        // Reusing a 30 byte buffer for a 20 byte grey charges 30.
        let claim = gate.admit(None, 20, 30);
        assert!(claim.is_ok());
        assert_eq!(gate.held(), (30, 0));
        drop(claim);
        assert_eq!(gate.held(), (0, 0));
        // Replacing a 30 byte buffer for a 60 byte grey charges 60, and
        // fits beside 40 held elsewhere because the 30 leave.
        assert!(gate.count_returned(30));
        let other = gate.reserve(40);
        let claim = gate.admit(None, 60, 30);
        assert!(claim.is_ok());
        assert_eq!(gate.held(), (100, 0));
        drop(claim);
        drop(other);
        assert_eq!(gate.held(), (0, 0));
    }

    #[test]
    fn a_refused_render_settles_to_what_the_worker_kept() -> Result<(), String> {
        let gate = small_gate();
        let Ok(mut claim) = gate.admit(None, 60, 0) else {
            return Err("the render was not admitted".to_owned());
        };
        // The refused render left a 60 byte buffer with the worker.
        gate.settle(&mut claim, 0, 60);
        assert_eq!(gate.held(), (0, 60));
        drop(claim);
        assert_eq!(gate.held(), (0, 60));
        gate.release_returned(60);
        assert_eq!(gate.held(), (0, 0));
        Ok(())
    }

    #[test]
    fn a_written_reply_is_kept_only_while_nobody_waits() -> Result<(), String> {
        let gate = small_gate();
        let Some(claim) = gate.reserve(40) else {
            return Err("no room".to_owned());
        };
        let ReturnedGrey::Keep(kept) = gate.return_reply(claim) else {
            return Err("the reply was not kept".to_owned());
        };
        assert_eq!(gate.held(), (0, 40));
        // The worker died before it took the buffer.
        drop(kept);
        assert_eq!(gate.held(), (0, 0));

        let Some(claim) = gate.reserve(40) else {
            return Err("no room".to_owned());
        };
        let waiter = gate.enter_wait(40);
        assert!(gate.has_waiters());
        let ReturnedGrey::Free(claim) = gate.return_reply(claim) else {
            return Err("the reply was kept while a request waited".to_owned());
        };
        // The bytes count until the buffer is freed and the claim drops.
        assert_eq!(gate.held(), (40, 0));
        drop(claim);
        assert_eq!(gate.held(), (0, 0));
        drop(waiter);
        assert!(!gate.has_waiters());
        assert_eq!(gate.waiting(), 0);
        Ok(())
    }
    #[test]
    fn a_grey_over_the_cap_is_admitted_only_alone() -> Result<(), String> {
        let gate = small_gate();
        // Beside 1 byte held, 150 is not admitted, by either door.
        let other = gate.reserve(1).ok_or("no room for 1")?;
        assert!(gate.reserve(150).is_none());
        assert!(gate.admit(None, 150, 0).is_err());
        drop(other);
        // Beside a returned buffer it is not admitted either.
        assert!(gate.count_returned(10));
        assert!(gate.reserve(150).is_none());
        gate.release_returned(10);
        // Alone it is, and nothing else enters while it is held.
        let big = gate
            .admit(None, 150, 0)
            .map_err(|_| "150 alone was refused")?;
        assert_eq!(gate.held(), (150, 0));
        assert!(gate.reserve(1).is_none());
        assert!(gate.reserve(150).is_none());
        assert!(gate.admit(None, 1, 0).is_err());
        // Written, it is freed rather than kept by its worker.
        let ReturnedGrey::Free(big) = gate.return_reply(big) else {
            return Err("a grey over the cap was kept".to_owned());
        };
        drop(big);
        assert_eq!(gate.held(), (0, 0));
        assert_eq!(gate.peak(), 150);
        Ok(())
    }

    #[test]
    fn waiters_are_admitted_in_arrival_order() -> Result<(), String> {
        let gate = small_gate();
        let held = gate.reserve(70).ok_or("no room for 70")?;
        // A grey over the cap waits first, then a grey of 20.
        let big = gate.enter_wait(150);
        let small = gate.enter_wait(20);
        // 20 fits beside 70, but it is not at the head.
        assert!(gate.wait_turn(&small, Duration::ZERO).is_none());
        // A request that is not waiting does not pass the queue either.
        assert!(gate.reserve(10).is_none());
        assert!(gate.admit(None, 10, 0).is_err());
        assert_eq!(gate.waiting(), 2);
        // The head waits for the 70 to go, then draws alone.
        assert!(gate.wait_turn(&big, Duration::ZERO).is_none());
        drop(held);
        let big_claim = gate
            .wait_turn(&big, Duration::ZERO)
            .ok_or("the head was not admitted alone")?;
        assert_eq!(gate.waiting(), 1);
        assert!(gate.wait_turn(&small, Duration::ZERO).is_none());
        drop(big_claim);
        let small_claim = gate
            .wait_turn(&small, Duration::ZERO)
            .ok_or("the second was not admitted")?;
        assert_eq!(gate.waiting(), 0);
        assert_eq!(gate.held(), (20, 0));
        drop(small_claim);
        drop(small);
        drop(big);
        assert_eq!(gate.held(), (0, 0));
        Ok(())
    }

    #[test]
    fn a_second_grey_over_the_cap_does_not_pass_the_queue() -> Result<(), String> {
        let gate = small_gate();
        let first = gate.enter_wait(150);
        // Nothing is held, but a grey over the cap waits, so a second one
        // that is not waiting is refused by either door and queues behind.
        assert!(gate.reserve(150).is_none());
        let second = gate
            .admit(None, 150, 0)
            .err()
            .ok_or("the second grey over the cap passed the queue")?;
        assert_eq!(gate.waiting(), 2);
        assert!(gate.wait_turn(&second, Duration::ZERO).is_none());
        let claim = gate
            .wait_turn(&first, Duration::ZERO)
            .ok_or("the first was not admitted")?;
        assert!(gate.wait_turn(&second, Duration::ZERO).is_none());
        drop(claim);
        let claim = gate
            .wait_turn(&second, Duration::ZERO)
            .ok_or("the second was not admitted after the first")?;
        assert_eq!(gate.peak(), 150);
        drop(claim);
        Ok(())
    }

    #[test]
    fn a_request_that_holds_room_is_never_queued() -> Result<(), String> {
        let gate = small_gate();
        let held = gate.reserve(40).ok_or("no room for 40")?;
        let waiter = gate.enter_wait(150);
        // A request that holds no room is refused while the queue is not
        // empty, though 20 fits beside 40.
        assert!(gate.admit(None, 20, 0).is_err());
        // The request holding 40 renders past the waiter.
        let rendered = gate
            .admit(Some(held), 40, 0)
            .map_err(|_| "the request holding room was queued")?;
        assert_eq!(gate.waiting(), 1);
        drop(rendered);
        drop(waiter);
        // With the queue empty a request that fits enters at once.
        let claim = gate.admit(None, 20, 0).map_err(|_| "20 was queued")?;
        assert_eq!(gate.waiting(), 0);
        drop(claim);
        Ok(())
    }

    #[test]
    fn a_place_that_leaves_the_head_lets_the_next_in() -> Result<(), String> {
        let gate = small_gate();
        let held = gate.reserve(60).ok_or("no room for 60")?;
        let head = gate.enter_wait(60);
        let next = gate.enter_wait(30);
        assert!(gate.wait_turn(&next, Duration::ZERO).is_none());
        // The head leaves without being admitted, as a timeout or a gone
        // client does.
        drop(head);
        assert_eq!(gate.waiting(), 1);
        let claim = gate
            .wait_turn(&next, Duration::ZERO)
            .ok_or("the next was not admitted")?;
        drop(claim);
        drop(held);
        Ok(())
    }
}
