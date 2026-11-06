//! Files and decoded frames held by one viewer process.
//!
//! One `Lru` accounts both. Encoded bytes leave by `Lru::remove` once every
//! frame of that file is held. The tier is `CacheTier::Decoded` because the
//! frames are what the budget is for. A decoded frame's sample buffer that
//! leaves the cache is parked on a free list keyed by its exact byte length
//! when cached bytes plus free bytes fit the budget, and is freed otherwise.
//! Cached frames and free buffers share one budget. A decode of that length
//! takes a parked buffer when there is one, and allocates otherwise.

use std::collections::{HashMap, HashSet, VecDeque};
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
    /// Posted bytes. The cache keeps this `Arc` until the frame is stored.
    Shared(Arc<PostedBytes>),
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
            Self::Frame(frame) => frame.samples.capacity() + size_of::<HeldFrame>(),
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

/// The grey cap in bytes, `P * N_max` at one grey byte per pixel.
pub(crate) const GREY_CAP: usize = PERMITS * N_MAX_PIXELS;

/// Every grey byte the process server holds, under one fixed cap.
///
/// Two kinds of grey buffer count. A reply that is not yet written holds its
/// grey from the moment its render is admitted, by capacity. Each decode
/// worker keeps at most one returned grey buffer to render into next. Their
/// sum never exceeds the cap, which is [`GREY_CAP`] for the server. A render
/// whose grey would exceed it waits, and a grey larger than the whole cap is
/// refused at once.
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
    /// Requests waiting for room. While any waits, no returned buffer is kept.
    waiting: usize,
}

impl GreyState {
    fn used(&self) -> usize {
        self.replies.saturating_add(self.returned)
    }
}

/// Why a render was not admitted under the grey cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GreyRefusal {
    /// No room now. The caller may wait for room.
    Full,
    /// The grey alone is larger than the cap, so no wait can admit it.
    TooLarge,
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

/// One request waiting for grey room. Drop ends the wait.
pub(crate) struct GreyWaiter<'a> {
    gate: &'a GreyGate,
}

impl Drop for GreyWaiter<'_> {
    fn drop(&mut self) {
        let mut state = lock_mutex(&self.gate.state);
        state.waiting = state.waiting.saturating_sub(1);
    }
}

impl GreyGate {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            state: Mutex::new(GreyState {
                replies: 0,
                returned: 0,
                waiting: 0,
            }),
            ready: Condvar::new(),
            cap,
        }
    }

    /// Reserve `len` grey bytes for a render that has not run yet.
    ///
    /// `None` means the cap has no room for them now.
    pub(crate) fn reserve(self: &Arc<Self>, len: usize) -> Option<GreyClaim> {
        let mut state = lock_mutex(&self.state);
        if state.used().saturating_add(len) > self.cap {
            return None;
        }
        state.replies = state.replies.saturating_add(len);
        Some(GreyClaim {
            gate: Arc::clone(self),
            bytes: len,
        })
    }

    /// Admit a render of `len` grey bytes into a worker buffer of `kept`
    /// bytes, which the returned count holds.
    ///
    /// The render reuses the buffer when it is large enough and replaces it
    /// otherwise, so the reply then holds `max(kept, len)` bytes and the
    /// buffer is no longer returned. `claim` is room this request already
    /// waited for, and it counts toward the admission. A refusal releases it.
    pub(crate) fn admit(
        self: &Arc<Self>,
        claim: Option<GreyClaim>,
        len: usize,
        kept: usize,
    ) -> Result<GreyClaim, GreyRefusal> {
        let mut claim = claim.unwrap_or(GreyClaim {
            gate: Arc::clone(self),
            bytes: 0,
        });
        if len > self.cap {
            return Err(GreyRefusal::TooLarge);
        }
        let charged = kept.max(len);
        let mut state = lock_mutex(&self.state);
        let after = state
            .used()
            .saturating_sub(claim.bytes)
            .saturating_sub(kept)
            .saturating_add(charged);
        if after > self.cap {
            drop(state);
            return Err(GreyRefusal::Full);
        }
        let before = state.used();
        let held = claim.disarm();
        state.replies = state.replies.saturating_sub(held).saturating_add(charged);
        state.returned = state.returned.saturating_sub(kept);
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
    /// request waits for room.
    pub(crate) fn return_reply(&self, mut claim: GreyClaim) -> ReturnedGrey {
        let mut state = lock_mutex(&self.state);
        if state.waiting > 0 {
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
        lock_mutex(&self.state).waiting > 0
    }

    /// Count one request as waiting until the guard drops.
    pub(crate) fn enter_wait(&self) -> GreyWaiter<'_> {
        let mut state = lock_mutex(&self.state);
        state.waiting = state.waiting.saturating_add(1);
        GreyWaiter { gate: self }
    }

    /// Grey bytes held by unwritten replies, then by returned buffers.
    pub(crate) fn held(&self) -> (usize, usize) {
        let state = lock_mutex(&self.state);
        (state.replies, state.returned)
    }

    /// Requests waiting for room now.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> usize {
        lock_mutex(&self.state).waiting
    }

    /// Count a returned buffer of `bytes` a test placed, when it fits.
    #[cfg(test)]
    pub(crate) fn count_returned(&self, bytes: usize) -> bool {
        let mut state = lock_mutex(&self.state);
        if state.used().saturating_add(bytes) > self.cap {
            return false;
        }
        state.returned = state.returned.saturating_add(bytes);
        true
    }

    /// Sleep until a release or `slice`, whichever first.
    pub(crate) fn wait_slice(&self, slice: Duration) {
        let guard = lock_mutex(&self.state);
        let _ = self
            .ready
            .wait_timeout(guard, slice)
            .unwrap_or_else(|poison| poison.into_inner());
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
                Some(Held::File(bytes)) => PreviewSource::Shared(Arc::clone(bytes)),
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
        let sample_cap = held.samples.capacity();
        let value = Arc::new(CachedFrame {
            inner: held,
            worker,
            retire,
            recycle: Some(self.recycle_tx.clone()),
        });
        let key = HeldKey::Frame(id, index);
        let mut state = self.lock_cache();
        state.drain_recycled();
        state.reserve_for_insert(sample_cap.saturating_add(size_of::<HeldFrame>()));
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

    /// Remove a posted file and its cached frames, as eviction would.
    #[cfg(test)]
    pub(crate) fn forget_file(&self, id: FileId, frames: u32) {
        for index in 0..frames {
            let _ = self.evict_frame_to_free_list(id, index);
        }
        let mut state = self.lock_cache();
        if state.lru.remove(&HeldKey::File(id)).is_some() {
            state.files = state.files.saturating_sub(1);
        }
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

fn frame_sample_cost(len: usize) -> usize {
    len.saturating_add(size_of::<HeldFrame>())
}

impl CacheState {
    fn drain_recycled(&mut self) {
        while let Ok(buffer) = self.recycle_rx.try_recv() {
            self.park_if_fits(buffer);
        }
    }

    fn charged(&self) -> usize {
        self.lru.pressure().used.saturating_add(self.free_bytes)
    }

    fn claim_buffer(&mut self, len: usize) -> Vec<u8> {
        self.drain_recycled();
        if let Some(buffer) = self.pop_free(len) {
            self.note_reused();
            let cost = buffer.capacity().saturating_add(size_of::<HeldFrame>());
            self.make_room_dropping(cost);
            return buffer;
        }
        let cost = frame_sample_cost(len);
        if self.charged().saturating_add(cost) <= self.limit {
            self.note_fresh();
            return Vec::new();
        }
        if let Some(buffer) = self.take_oldest_frame_buffer(len) {
            self.park_if_fits(buffer);
            if let Some(buffer) = self.pop_free(len) {
                self.note_reused();
                return buffer;
            }
        }
        self.make_room_dropping(cost);
        self.note_fresh();
        Vec::new()
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
    /// because this room is for a buffer the caller already holds.
    fn make_room_dropping(&mut self, cost: usize) {
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

    /// Evict one least recently used entry. The caller parks or drops it.
    fn evict_oldest(&mut self) -> Vec<Held> {
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
        let mut evicted = Vec::new();
        for (key, held) in admission.evicted {
            self.forget_frame(&key);
            match key {
                HeldKey::File(_) => self.files = self.files.saturating_sub(1),
                HeldKey::Frame(_, _) => self.frames = self.frames.saturating_sub(1),
            }
            evicted.push(held);
        }
        evicted
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
                self.forget_frame(&evicted_key);
                match evicted_key {
                    HeldKey::File(_) => self.files = self.files.saturating_sub(1),
                    HeldKey::Frame(_, _) => self.frames = self.frames.saturating_sub(1),
                }
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
            self.forget_frame(&evicted_key);
            match evicted_key {
                HeldKey::File(_) => self.files = self.files.saturating_sub(1),
                HeldKey::Frame(_, _) => self.frames = self.frames.saturating_sub(1),
            }
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
        assert_eq!(gate.admit(waited, 50, 0).err(), Some(GreyRefusal::Full));
        assert_eq!(gate.held(), (80, 0));
        // A grey larger than the cap is refused whatever was waited for.
        let waited = gate.reserve(10);
        assert_eq!(
            gate.admit(waited, 101, 0).err(),
            Some(GreyRefusal::TooLarge)
        );
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
        let waiter = gate.enter_wait();
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
}
