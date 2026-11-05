//! Files and decoded frames held by one viewer process.
//!
//! One `Lru` accounts both. Encoded bytes leave by `Lru::remove` once every
//! frame of that file is held. The tier is `CacheTier::Decoded` because the
//! frames are what the budget is for.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use ocelli_cache::{Admission, Budgeted, CacheTier, Lru, Pressure};
use ocelli_core::Stored;

use crate::preview::HeldFrame;

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
    Hit(Arc<HeldFrame>),
    /// The posted bytes, shared with the cache so a decode does not copy them.
    Miss(Arc<Vec<u8>>),
}

/// A preview's view of one id. `Owned` is the posted buffer with no other
/// owner, so a single native frame can become the held samples in place.
pub(crate) enum PreviewSource {
    /// The id is unknown, or the bytes and this frame are both gone.
    Gone,
    /// The decoded frame.
    Hit(Arc<HeldFrame>),
    /// Posted bytes another caller still holds.
    Shared(Arc<Vec<u8>>),
    /// Posted bytes this caller now owns. The cache no longer has them.
    Owned(Vec<u8>),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HeldKey {
    File(FileId),
    Frame(FileId, u32),
}

enum Held {
    /// Posted bytes. Shared, so a lookup does not copy the buffer.
    File(Arc<Vec<u8>>),
    Frame(Arc<HeldFrame>),
}

impl Budgeted for Held {
    fn bytes(&self) -> usize {
        match self {
            Self::File(bytes) => bytes.capacity(),
            Self::Frame(frame) => frame.samples.capacity() + size_of::<HeldFrame>(),
        }
    }
}

struct CacheState {
    lru: Lru<HeldKey, Held>,
    files: usize,
    frames: usize,
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
}

pub(crate) struct WorkspaceGuard<'a> {
    pool: &'a PermitPool,
    pub(crate) workspace: Workspace,
}

/// Spare posted buffers. A dropped file and the next body of the same size
/// share one allocation, so the resident set does not keep both size classes.
struct RecyclePool {
    buffers: Vec<Vec<u8>>,
    held: usize,
}

const RECYCLE_BUFFERS: usize = 4;
const RECYCLE_BYTES: usize = 64 * 1024 * 1024;
const RECYCLE_MIN: usize = 4096;
const BODY_QUANTUM: usize = 16 * 1024;

/// Round a posted body up to a 16 KiB multiple.
///
/// Files in one series differ by the width of a UID and an instance number.
/// This host keeps a freed allocation class resident, so those lengths have
/// to share one class or the resident set keeps every class.
fn body_class(need: usize) -> usize {
    need.saturating_add(BODY_QUANTUM - 1) / BODY_QUANTUM * BODY_QUANTUM
}

impl Drop for WorkspaceGuard<'_> {
    fn drop(&mut self) {
        let workspace = std::mem::take(&mut self.workspace);
        let mut state = lock_mutex(&self.pool.idle);
        state.idle.push(workspace);
        self.pool.ready.notify_one();
    }
}

fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// One process-local cache of posted files and decoded frames.
pub struct Viewer {
    generation: u64,
    serial: AtomicU64,
    cache: Mutex<CacheState>,
    permits: PermitPool,
    recycled: Mutex<RecyclePool>,
    connections: AtomicUsize,
    connection_cap: usize,
}

impl Viewer {
    /// A viewer with the default connection cap of 32.
    #[must_use]
    pub fn new(config: ViewerConfig) -> Self {
        Self::with_connection_cap(config, CONNECTION_CAP)
    }

    /// A viewer with an explicit connection cap. Zero refuses every connection.
    #[must_use]
    pub fn with_connection_cap(config: ViewerConfig, cap: usize) -> Self {
        let mut idle = Vec::with_capacity(PERMITS);
        for _ in 0..PERMITS {
            idle.push(Workspace::default());
        }
        Self {
            generation: mix_generation(),
            serial: AtomicU64::new(0),
            cache: Mutex::new(CacheState {
                lru: Lru::new(CacheTier::Decoded, config.cache_bytes),
                files: 0,
                frames: 0,
            }),
            permits: PermitPool {
                idle: Mutex::new(PoolState {
                    idle,
                    spare_grey: Vec::new(),
                }),
                ready: Condvar::new(),
            },
            recycled: Mutex::new(RecyclePool {
                buffers: Vec::new(),
                held: 0,
            }),
            connections: AtomicUsize::new(0),
            connection_cap: cap,
        }
    }

    /// The cache pressure reading. Not a use.
    #[must_use]
    pub fn pressure(&self) -> Pressure {
        self.census().0
    }

    pub(crate) fn census(&self) -> (Pressure, usize, usize) {
        let state = self.lock_cache();
        (state.lru.pressure(), state.files, state.frames)
    }

    pub(crate) fn insert_file(&self, bytes: Vec<u8>) -> Option<FileId> {
        let id = self.allocate_id();
        let key = HeldKey::File(id);
        let mut state = self.lock_cache();
        let admission = state.lru.insert(key, Held::File(Arc::new(bytes)));
        let kept = admission.refused.is_none();
        account_insert(&mut state, key, &admission);
        let spare = spare_buffers(admission);
        drop(state);
        self.keep_buffers(spare);
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

    /// Frame, shared bytes, or the unique posted buffer removed from the cache.
    pub(crate) fn preview_source(&self, id: FileId, frame: u32) -> PreviewSource {
        let mut state = self.lock_cache();
        if let Some(Held::Frame(held)) = state.lru.get(&HeldKey::Frame(id, frame)) {
            return PreviewSource::Hit(Arc::clone(held));
        }
        let key = HeldKey::File(id);
        let unique = matches!(
            state.lru.get(&key),
            Some(Held::File(bytes)) if Arc::strong_count(bytes) == 1
        );
        if unique && let Some(Held::File(bytes)) = state.lru.remove(&key) {
            state.files = state.files.saturating_sub(1);
            match Arc::try_unwrap(bytes) {
                Ok(owned) => return PreviewSource::Owned(owned),
                Err(shared) => {
                    let admission = state.lru.insert(key, Held::File(shared));
                    let kept = admission.refused.is_none();
                    account_insert(&mut state, key, &admission);
                    let returned = if kept {
                        match state.lru.get(&key) {
                            Some(Held::File(bytes)) => PreviewSource::Shared(Arc::clone(bytes)),
                            Some(Held::Frame(_)) | None => PreviewSource::Gone,
                        }
                    } else {
                        PreviewSource::Gone
                    };
                    let spare = spare_buffers(admission);
                    drop(state);
                    self.keep_buffers(spare);
                    return returned;
                }
            }
        }
        match state.lru.get(&key) {
            Some(Held::File(bytes)) => PreviewSource::Shared(Arc::clone(bytes)),
            Some(Held::Frame(_)) | None => PreviewSource::Gone,
        }
    }

    /// Put posted bytes back under `id` after a decode that still needs them.
    pub(crate) fn restore_file(&self, id: FileId, bytes: Vec<u8>) {
        let key = HeldKey::File(id);
        let mut state = self.lock_cache();
        if state.lru.contains_key(&key) {
            drop(state);
            self.keep_buffers(vec![bytes]);
            return;
        }
        let admission = state.lru.insert(key, Held::File(Arc::new(bytes)));
        account_insert(&mut state, key, &admission);
        let spare = spare_buffers(admission);
        drop(state);
        self.keep_buffers(spare);
    }

    pub(crate) fn store_frame(&self, id: FileId, index: u32, held: HeldFrame) -> Arc<HeldFrame> {
        let frames = held.frames;
        let value = Arc::new(held);
        let key = HeldKey::Frame(id, index);
        let mut state = self.lock_cache();
        let admission = state.lru.insert(key, Held::Frame(Arc::clone(&value)));
        let kept = admission.refused.is_none();
        account_insert(&mut state, key, &admission);
        let removed = if kept {
            take_encoded_if_ready(&mut state, id, frames)
        } else {
            None
        };
        let mut spare = spare_buffers(admission);
        if let Some(held) = removed
            && let Some(buffer) = unique_bytes(held)
        {
            spare.push(buffer);
        }
        drop(state);
        self.keep_buffers(spare);
        value
    }

    pub(crate) fn file_bytes(&self, id: FileId) -> Option<Arc<Vec<u8>>> {
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

    /// A body buffer of at least `need` bytes. A recycled buffer is used only
    /// when its capacity is the same size class, so a radiograph is not reused
    /// as a CT file.
    pub(crate) fn take_buffer(&self, need: usize) -> Vec<u8> {
        if need < RECYCLE_MIN {
            let mut body = Vec::new();
            body.reserve_exact(need);
            return body;
        }
        let class = body_class(need);
        let mut pool = lock_mutex(&self.recycled);
        let found = pool.buffers.iter().enumerate().find_map(|(index, buf)| {
            let cap = buf.capacity();
            // A class may come back a quantum above the request. A radiograph
            // is many quanta above a CT file, so the two do not meet.
            let limit = class.saturating_add(BODY_QUANTUM);
            (cap >= need && cap <= limit).then_some(index)
        });
        if let Some(index) = found {
            let mut buffer = pool.buffers.swap_remove(index);
            pool.held = pool.held.saturating_sub(buffer.capacity());
            buffer.clear();
            return buffer;
        }
        drop(pool);
        let mut body = Vec::new();
        body.reserve_exact(class);
        body
    }

    pub(crate) fn recycle_buffer(&self, buffer: Vec<u8>) {
        self.keep_buffers(vec![buffer]);
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
        lock_mutex(&self.cache)
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
        Some(WorkspaceGuard {
            pool: &self.permits,
            workspace,
        })
    }

    fn keep_buffers(&self, buffers: Vec<Vec<u8>>) {
        let mut pool = lock_mutex(&self.recycled);
        for mut buffer in buffers {
            buffer.clear();
            let cap = buffer.capacity();
            if cap < RECYCLE_MIN {
                continue;
            }
            if pool.buffers.len() >= RECYCLE_BUFFERS
                || pool.held.saturating_add(cap) > RECYCLE_BYTES
            {
                continue;
            }
            pool.held = pool.held.saturating_add(cap);
            pool.buffers.push(buffer);
        }
    }
}

fn account_insert(state: &mut CacheState, key: HeldKey, admission: &Admission<HeldKey, Held>) {
    if admission.refused.is_some() {
        return;
    }
    for (evicted, _) in &admission.evicted {
        match evicted {
            HeldKey::File(_) => state.files = state.files.saturating_sub(1),
            HeldKey::Frame(_, _) => state.frames = state.frames.saturating_sub(1),
        }
    }
    if admission.displaced.is_none() {
        match key {
            HeldKey::File(_) => state.files = state.files.saturating_add(1),
            HeldKey::Frame(_, _) => state.frames = state.frames.saturating_add(1),
        }
    }
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

fn unique_bytes(held: Held) -> Option<Vec<u8>> {
    match held {
        Held::File(bytes) => Arc::try_unwrap(bytes).ok(),
        Held::Frame(frame) => Arc::try_unwrap(frame).ok().map(|frame| frame.samples),
    }
}

fn spare_buffers(admission: Admission<HeldKey, Held>) -> Vec<Vec<u8>> {
    let mut spare = Vec::new();
    for (_, held) in admission.evicted {
        if let Some(buffer) = unique_bytes(held) {
            spare.push(buffer);
        }
    }
    if let Some(held) = admission.displaced
        && let Some(buffer) = unique_bytes(held)
    {
        spare.push(buffer);
    }
    if let Some((_, held)) = admission.refused
        && let Some(buffer) = unique_bytes(held)
    {
        spare.push(buffer);
    }
    spare
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
    fn a_few_header_bytes_do_not_open_a_second_body_class() {
        let viewer = Viewer::new(ViewerConfig { cache_bytes: 1024 });
        // The bench CT files are 524822 and 524832 bytes. The gap is the UID
        // and the instance number. 17400538 is the bench radiograph.
        let short_len = 524_822_usize;
        let long_len = 524_832_usize;
        let radiograph = 17_400_538_usize;
        let short = viewer.take_buffer(short_len);
        let long = viewer.take_buffer(long_len);
        assert!(short.capacity() >= long_len);
        assert_eq!(short.capacity(), long.capacity());
        assert_eq!(short.capacity() % (16 * 1024), 0);
        let dx = viewer.take_buffer(radiograph);
        assert!(dx.capacity() >= radiograph);
        assert_ne!(dx.capacity(), short.capacity());
        viewer.recycle_buffer(short);
        let reused = viewer.take_buffer(long_len);
        assert_eq!(reused.capacity(), long.capacity());
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
        let held = Held::Frame(Arc::new(minimal_frame(samples, 512, 512)?));
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
        let held = Held::File(Arc::new(spare));
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
        let cost = Held::Frame(Arc::new(minimal_frame(vec![0_u8; 32], 1, 1)?)).bytes();
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
        let (pressure, _, frames) = viewer.census();
        if pressure.used != cost || pressure.used > pressure.budget || frames != 1 {
            return Err(format!(
                "after eviction used {} budget {} frames {frames}",
                pressure.used, pressure.budget
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
}
