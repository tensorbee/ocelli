//! One session, one outstanding bulk allocation, and the copied event drain.
//!
//! The write address and the lifetime ticket are different `u32` values.
//! This file does not read an arbitrary address and it contains no `unsafe`.
//! On a 64-bit host the write address is a counter, because a real pointer
//! does not fit in `u32`. On wasm32 the write address is the allocation's
//! address. A later allocation may reuse that address. The ticket is what
//! `commit_frame` and `apply_commands` accept, and a ticket from an earlier
//! allocation does not authorize the new one. `release_bulk` drops the
//! current ticket and publishes nothing.

use ocelli_cache::{
    CacheEvent, CacheEvents, CommittedFrame, CommittedSource, ContentId, FrameKey, FrameLookup,
    ImageCache, Pressure, PressureBand, SeriesId, TelemetryEvent, TierCounters, TierTelemetry,
};
use ocelli_core::Stored;
use ocelli_render::{CpuRaster, RasterError, RasterGrid, RasterQuality, RasterResult};
use ocelli_viewport::commands::{STATE_BYTES, StackViewport};
use ocelli_viewport::ring::EventRing;
use ocelli_viewport::scroll::StackSequence;

/// 64 MiB. A larger `alloc` is refused and leaves any outstanding buffer.
pub const MAX_BULK_BYTES: u32 = 64 * 1024 * 1024;

/// The four integers `commit_frame` stores with the accepted bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameMeta {
    /// Viewport id. It must match the session's stack viewport.
    pub viewport: u32,
    /// Columns of the committed frame.
    pub columns: u32,
    /// Rows of the committed frame.
    pub rows: u32,
    /// Generation stored on the viewport and copied into a Presented event.
    pub generation: u32,
}

/// Why a commit wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// No outstanding allocation, or the token is not the one `alloc` returned.
    Stale,
    /// The length is not the outstanding buffer's length.
    Length,
    /// [`FrameMeta::viewport`] is not this session's viewport.
    Viewport,
    /// A frame names a stale stack epoch or a different selected index.
    Selection,
}

#[derive(Debug)]
struct Bulk {
    ticket: u32,
    address: u32,
    bytes: Vec<u8>,
}

/// Shade used by a draw that does not name one.
///
/// Monochrome is the default. RGB is remembered only after an interleaved RGB
/// draw returns Ok. A refused request leaves this value unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CpuShade {
    Mono,
    Rgb,
}

/// The stack viewport, its one bulk slot, and its event ring.
#[derive(Debug)]
pub struct Session {
    viewport: StackViewport,
    bulk: Option<Bulk>,
    next_ticket: u32,
    #[cfg(not(target_arch = "wasm32"))]
    next_address: u32,
    last_frame: Vec<u8>,
    stored_samples: Vec<Stored>,
    last_meta: Option<FrameMeta>,
    ring: EventRing,
    cpu: Option<CpuRaster>,
    shade: CpuShade,
    /// Drawn bytes. A reduced target occupies the prefix. The tail is zero.
    cpu_rgba: Vec<u8>,
    /// Device-sized expansion of `cpu_rgba`. The canvas blits this buffer.
    cpu_present: Vec<u8>,
    stack_epoch: u32,
    /// Last `snapshot` refusal. Zero after a successful snapshot.
    snapshot_code: u32,
    /// The session's own decoded frame cache. The budget is the caller's and
    /// the default is zero, which is off: a caller that never sets one gets
    /// today's behaviour exactly.
    frames: ImageCache<CommittedFrame>,
    /// The last pressure band reported through the ring. F-034's caller
    /// keeps the band so a crossing is one event and not a stream.
    last_band: PressureBand,
    /// Evictions and refusals since the previous pressure reading, F-034's
    /// `TierCounters`. Every reading takes them, so a reading resets them
    /// and the next one starts from zero.
    counters: TierCounters,
    /// The stack's keys in stored order, parsed by `configure_stack`, so a
    /// residency query or a show can name any index and not only the
    /// selected one.
    stack_keys: Vec<FrameKey>,
}

impl Session {
    /// Own `viewport`. No bulk allocation is outstanding.
    #[must_use]
    pub fn new(viewport: StackViewport) -> Self {
        Self {
            viewport,
            bulk: None,
            next_ticket: 0x1000_0001,
            #[cfg(not(target_arch = "wasm32"))]
            next_address: 1,
            last_frame: Vec::new(),
            stored_samples: Vec::new(),
            last_meta: None,
            ring: EventRing::new(),
            cpu: None,
            shade: CpuShade::Mono,
            cpu_rgba: Vec::new(),
            cpu_present: Vec::new(),
            stack_epoch: 0,
            snapshot_code: 0,
            frames: ImageCache::new(0),
            last_band: PressureBand::Low,
            counters: TierCounters::default(),
            stack_keys: Vec::new(),
        }
    }

    /// The viewport commands mutate.
    #[must_use]
    pub const fn viewport(&self) -> &StackViewport {
        &self.viewport
    }

    /// Bytes accepted by the last successful commit. Empty before that.
    #[must_use]
    pub fn last_frame(&self) -> &[u8] {
        &self.last_frame
    }

    /// Metadata accepted by the last successful commit.
    #[must_use]
    pub const fn last_meta(&self) -> Option<FrameMeta> {
        self.last_meta
    }

    /// Reserve `len` bytes and return the write address.
    ///
    /// `0` means refused and no slot was created. A second call while one
    /// allocation is outstanding returns `0` and keeps the first. The
    /// authorization for the slot is [`Self::bulk_ticket`], not this address.
    #[must_use]
    pub fn alloc(&mut self, len: u32) -> u32 {
        if self.bulk.is_some() || len > MAX_BULK_BYTES {
            return 0;
        }
        let Ok(size) = usize::try_from(len) else {
            return 0;
        };
        let bytes = vec![0_u8; size];
        let address = self.address_for(&bytes);
        if address == 0 {
            return 0;
        }
        let ticket = self.fresh_ticket();
        self.bulk = Some(Bulk {
            ticket,
            address,
            bytes,
        });
        address
    }

    /// Lifetime ticket for the outstanding allocation. `0` when there is none.
    #[must_use]
    pub const fn bulk_ticket(&self) -> u32 {
        match &self.bulk {
            Some(bulk) => bulk.ticket,
            None => 0,
        }
    }

    /// The outstanding bytes at `address`, so a native test can fill them.
    /// Wasm fills the same bytes through the write address.
    pub fn bulk_mut(&mut self, address: u32) -> Option<&mut [u8]> {
        let bulk = self.bulk.as_mut()?;
        if address == 0 || bulk.address != address {
            return None;
        }
        Some(bulk.bytes.as_mut_slice())
    }

    /// Accept the outstanding bytes when `ticket` and `len` match, then drop
    /// the slot. A mismatched length or viewport drops that slot. A ticket
    /// that is not the outstanding one leaves the slot in place. A second
    /// commit, with nothing outstanding, returns [`SessionError::Stale`] and
    /// does not clear the last accepted frame.
    ///
    /// # Errors
    ///
    /// [`SessionError::Stale`], [`SessionError::Length`], or
    /// [`SessionError::Viewport`].
    pub fn commit_frame(
        &mut self,
        ticket: u32,
        len: u32,
        meta: FrameMeta,
    ) -> Result<(), SessionError> {
        if !self.ticket_is_current(ticket) {
            return Err(SessionError::Stale);
        }
        if !self.viewport.scroll().is_empty() {
            let _released = self.release_bulk(ticket);
            return Err(SessionError::Selection);
        }
        self.commit_current_frame(ticket, len, meta)
    }

    /// Commit only the selected immutable frame in the currently installed stack.
    ///
    /// # Errors
    /// [`SessionError`] for a foreign ticket, length, viewport or stale selection.
    pub fn commit_selected_frame(
        &mut self,
        ticket: u32,
        len: u32,
        meta: FrameMeta,
        index: u32,
        epoch: u32,
    ) -> Result<(), SessionError> {
        if !self.ticket_is_current(ticket) {
            return Err(SessionError::Stale);
        }
        if epoch == 0 || epoch != self.stack_epoch || self.viewport.scroll().index() != Some(index)
        {
            let _released = self.release_bulk(ticket);
            return Err(SessionError::Selection);
        }
        self.commit_current_frame(ticket, len, meta)
    }

    /// Install 16-byte little-endian FrameKeys and return a nonzero epoch.
    ///
    /// # Errors
    /// [`SessionError::Stale`] or [`SessionError::Length`]. Refusal is atomic.
    pub fn configure_stack(&mut self, ticket: u32, len: u32) -> Result<u32, SessionError> {
        if !self.ticket_is_current(ticket) {
            return Err(SessionError::Stale);
        }
        let Some(bulk) = self.bulk.take() else {
            return Err(SessionError::Stale);
        };
        if len == 0
            || usize::try_from(len).ok() != Some(bulk.bytes.len())
            || !bulk.bytes.len().is_multiple_of(16)
        {
            return Err(SessionError::Length);
        }
        let mut keys = Vec::with_capacity(bulk.bytes.len() / 16);
        for chunk in bulk.bytes.chunks_exact(16) {
            let series = u64::from_le_bytes(
                chunk
                    .get(..8)
                    .ok_or(SessionError::Length)?
                    .try_into()
                    .map_err(|_| SessionError::Length)?,
            );
            let instance = u32::from_le_bytes(
                chunk
                    .get(8..12)
                    .ok_or(SessionError::Length)?
                    .try_into()
                    .map_err(|_| SessionError::Length)?,
            );
            let frame = u32::from_le_bytes(
                chunk
                    .get(12..16)
                    .ok_or(SessionError::Length)?
                    .try_into()
                    .map_err(|_| SessionError::Length)?,
            );
            keys.push(FrameKey::new(SeriesId(series), instance, frame));
        }
        self.viewport.replace_stack(StackSequence::new(&keys));
        self.stack_keys = keys;
        self.stack_epoch = next_nonzero(self.stack_epoch);
        self.clear_selected_source();
        self.note_command_changed();
        Ok(self.stack_epoch)
    }

    fn clear_selected_source(&mut self) {
        self.last_meta = None;
        self.last_frame.clear();
        self.stored_samples.clear();
        self.viewport
            .set_loading(!self.viewport.scroll().is_empty());
    }

    fn commit_current_frame(
        &mut self,
        ticket: u32,
        len: u32,
        meta: FrameMeta,
    ) -> Result<(), SessionError> {
        if !self.ticket_is_current(ticket) {
            return Err(SessionError::Stale);
        }
        let Some(bulk) = self.bulk.take() else {
            return Err(SessionError::Stale);
        };
        let Ok(size) = usize::try_from(len) else {
            return Err(SessionError::Length);
        };
        if size != bulk.bytes.len() {
            return Err(SessionError::Length);
        }
        if meta.viewport != u32::from(self.viewport.id()) {
            return Err(SessionError::Viewport);
        }
        self.remember_frame(&bulk.bytes);
        self.last_meta = Some(meta);
        self.viewport.set_loading(false);
        self.adopt_cpu_source(meta.generation);
        Ok(())
    }

    /// Move the session frame cache's budget and emit the evictions, then
    /// one budget-changed event, then a pressure event if the band crossed.
    ///
    /// F-034's order: the caller emits the returned evictions first and a
    /// `BudgetChanged` after them. The count and byte total on the event are
    /// the evictions this call returned. Returns 1.
    pub fn set_frame_budget(&mut self, bytes: u32) -> u32 {
        let budget = usize::try_from(bytes).unwrap_or(0);
        let events = self.frames.set_budget(budget);
        let mut evicted = 0_u32;
        let mut evicted_bytes = 0_u64;
        for event in events.events() {
            if let CacheEvent::Evicted { bytes: left, .. } = *event {
                evicted = evicted.saturating_add(1);
                evicted_bytes =
                    evicted_bytes.saturating_add(u64::try_from(left).unwrap_or(u64::MAX));
            }
        }
        self.emit(&events);
        self.note_counters(&events);
        let pressure = self.frames.pressure();
        let changed = TelemetryEvent::BudgetChanged {
            tier: pressure.tier,
            budget: pressure.budget,
            evicted,
            evicted_bytes,
            dropped: self.ring.dropped(),
        };
        self.ring.push_with_seq(|seq| changed.encode(seq));
        self.emit_band();
        1
    }

    /// Commit the selected frame of the current epoch and cache it under
    /// the stack key at `index` with `content` naming the bytes.
    ///
    /// The refusal is `commit_selected_frame`'s, and a refused commit
    /// caches nothing. A commit whose sample count is zero, or a cache
    /// whose budget is zero, is displayed and not cached.
    ///
    /// # Errors
    ///
    /// [`SessionError`] as `commit_selected_frame` refuses.
    pub fn commit_cached_frame(
        &mut self,
        ticket: u32,
        len: u32,
        meta: FrameMeta,
        index: u32,
        epoch: u32,
        content: u64,
    ) -> Result<(), SessionError> {
        self.commit_selected_frame(ticket, len, meta, index, epoch)?;
        self.cache_committed_frame(index, meta, content);
        Ok(())
    }

    /// Insert the just-committed samples under the stack key at `index`.
    ///
    /// The samples exist only when the committed length was a multiple of
    /// four, and a budget of zero is a cache that is off. Both leave the
    /// display as `commit_selected_frame` made it and emit nothing.
    fn cache_committed_frame(&mut self, index: u32, meta: FrameMeta, content: u64) {
        if self.stored_samples.is_empty() || self.frames.pressure().budget == 0 {
            return;
        }
        let Some(key) = self.stack_key_at(index) else {
            return;
        };
        let committed = CommittedFrame::new(meta.columns, meta.rows, self.stored_samples.clone());
        let events = self.frames.insert(key, ContentId(content), committed);
        self.emit(&events);
        self.note_counters(&events);
        self.emit_band();
    }

    /// The stack's key at `index`, or `None` when the index is out of range
    /// or wider than a `usize`.
    fn stack_key_at(&self, index: u32) -> Option<FrameKey> {
        let index = usize::try_from(index).ok()?;
        self.stack_keys.get(index).copied()
    }

    /// Whether the stack key at `index` holds a frame committed from bytes
    /// whose identity is `content` at `columns` by `rows`.
    ///
    /// A query is not a use: it moves no recency and removes no mismatch.
    /// A stale epoch answers false.
    #[must_use]
    pub fn frame_resident(
        &self,
        index: u32,
        epoch: u32,
        content: u64,
        columns: u32,
        rows: u32,
    ) -> bool {
        if epoch == 0 || epoch != self.stack_epoch {
            return false;
        }
        let Some(key) = self.stack_key_at(index) else {
            return false;
        };
        self.frames.is_resident(
            &key,
            &CommittedSource {
                content: ContentId(content),
                columns,
                rows,
            },
        )
    }

    /// Show the cached frame under the stack key at `index` when it answers
    /// for `content` at `columns` by `rows`.
    ///
    /// `index` must be the viewport's selected one, the same rule
    /// `commit_selected_frame` commits under, because the displayed frame is
    /// the slice the state, the scroll bar and any measurement say it is. A
    /// different index is the [`SessionError::Selection`] refusal with
    /// nothing drawn and the loading flag as it was. A match adopts the
    /// frame exactly as `commit_current_frame` does, with generation 0. A
    /// mismatch is F-224's stale removal and leaves the displayed frame
    /// unchanged. A miss changes nothing. Returns 1 when the frame was
    /// shown and 0 on a miss, a stale entry or a wrong epoch.
    ///
    /// # Errors
    ///
    /// [`SessionError::Selection`] when `index` is not the selected index.
    pub fn show_resident(
        &mut self,
        index: u32,
        epoch: u32,
        content: u64,
        columns: u32,
        rows: u32,
    ) -> Result<u32, SessionError> {
        if epoch == 0 || epoch != self.stack_epoch {
            return Ok(0);
        }
        if self.viewport.scroll().index() != Some(index) {
            return Err(SessionError::Selection);
        }
        let Some(key) = self.stack_key_at(index) else {
            return Ok(0);
        };
        let source = CommittedSource {
            content: ContentId(content),
            columns,
            rows,
        };
        let shown = match self.frames.get(&key, &source) {
            FrameLookup::Hit(frame) => {
                // The two buffers are reused, not replaced, so a show
                // allocates nothing on a frame of the size already held.
                let samples = frame.samples();
                self.stored_samples.clear();
                self.stored_samples.extend_from_slice(samples);
                self.last_frame.clear();
                for stored in samples {
                    self.last_frame.extend_from_slice(&stored.0.to_le_bytes());
                }
                self.last_meta = Some(FrameMeta {
                    viewport: u32::from(self.viewport.id()),
                    columns,
                    rows,
                    generation: 0,
                });
                self.viewport.set_loading(false);
                true
            }
            FrameLookup::Stale { .. } => {
                // The removal changed what is held, so the band may have
                // crossed.
                self.emit_band();
                false
            }
            FrameLookup::Miss => false,
        };
        if shown {
            self.adopt_cpu_source(0);
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// Release every cached frame of `series` and push the release event.
    ///
    /// The displayed frame is outside the cache and stays. Returns the
    /// frame count released.
    pub fn release_series(&mut self, series: u64) -> u32 {
        let events = self.frames.release_series(SeriesId(series));
        let mut released = 0_u32;
        for event in events.events() {
            if let CacheEvent::Released { frames, .. } = *event {
                released = frames;
            }
        }
        self.emit(&events);
        self.emit_band();
        released
    }

    /// Push one pressure reading on demand. F-034: a requested reading uses
    /// the pressure event too. Returns 1.
    pub fn request_frame_pressure(&mut self) -> u32 {
        let event = self.pressure_reading();
        self.ring.push_with_seq(|seq| event.encode(seq));
        1
    }

    /// Hash the outstanding slot as the SOP Instance UID, the transfer
    /// syntax UID and the encoded bytes, and drop the slot.
    ///
    /// `uid_len` and `ts_len` are the byte lengths of the first two fields.
    /// A foreign ticket leaves the slot. Any other refusal drops it and
    /// answers `None`, which is a refusal and not an id of zero, because 0
    /// is a legitimate id.
    #[must_use]
    pub fn content_id(&mut self, ticket: u32, len: u32, uid_len: u32, ts_len: u32) -> Option<u64> {
        if !self.ticket_is_current(ticket) {
            return None;
        }
        let bulk = self.bulk.take()?;
        if usize::try_from(len).ok() != Some(bulk.bytes.len()) {
            return None;
        }
        let uid = usize::try_from(uid_len).ok()?;
        let ts = usize::try_from(ts_len).ok()?;
        let split = uid.checked_add(ts)?;
        if split > bulk.bytes.len() {
            return None;
        }
        let (uid_bytes, rest) = bulk.bytes.split_at(uid);
        let (ts_bytes, encoded) = rest.split_at(ts);
        Some(ContentId::of(uid_bytes, ts_bytes, encoded).0)
    }

    /// The frame cache's own pressure reading. Native tests, no export.
    #[must_use]
    pub fn frame_cache_pressure(&self) -> Pressure {
        self.frames.pressure()
    }

    /// Push each event in the ring in the order the cache returned them.
    fn emit(&mut self, events: &CacheEvents) {
        for event in events.events() {
            self.ring.push_with_seq(|seq| event.encode(seq));
        }
    }

    /// Compare the frame cache's band with the last one reported and push
    /// one pressure event on a crossing. F-034: the caller keeps the band.
    fn emit_band(&mut self) {
        let band = self.frames.pressure().band();
        if band == self.last_band {
            return;
        }
        self.last_band = band;
        let event = self.pressure_reading();
        self.ring.push_with_seq(|seq| event.encode(seq));
    }

    /// Fold the events one cache call returned into [`Self::counters`].
    ///
    /// F-034's `TierCounters::record` folds an `Admission`, which the
    /// session never holds: `ImageCache` converts it into `CacheEvents`
    /// first. The two arms mirror that fold, saturated the same way. A
    /// displaced value and a released series are neither an eviction nor a
    /// refusal, and a stale removal emits no event at all, so none of the
    /// three is counted.
    fn note_counters(&mut self, events: &CacheEvents) {
        for event in events.events() {
            match *event {
                CacheEvent::Evicted { bytes, .. } => {
                    self.counters.evicted = self.counters.evicted.saturating_add(1);
                    self.counters.evicted_bytes = self
                        .counters
                        .evicted_bytes
                        .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
                }
                CacheEvent::Refused { .. } => {
                    self.counters.refused = self.counters.refused.saturating_add(1);
                }
                CacheEvent::Displaced { .. } | CacheEvent::Released { .. } => {}
            }
        }
    }

    /// One pressure reading as F-034's event. `evicted`, `evicted_bytes`
    /// and `refused` are the counts since the previous reading, kept in
    /// [`Self::counters`] and taken here, so this reading resets them and
    /// the next one starts from zero. `entries` is the live count.
    fn pressure_reading(&mut self) -> TelemetryEvent {
        let pressure = self.frames.pressure();
        let counters = self.counters.take();
        TelemetryEvent::Pressure {
            tier: pressure.tier,
            telemetry: TierTelemetry {
                pressure,
                entries: u32::try_from(self.frames.len()).unwrap_or(u32::MAX),
                evicted: counters.evicted,
                evicted_bytes: counters.evicted_bytes,
                refused: counters.refused,
            },
        }
    }

    /// Read the outstanding buffer as one command batch.
    ///
    /// The current ticket drops the slot. Any other ticket returns 0 and
    /// leaves the slot. Success returns the command count. An empty buffer
    /// returns 0 and does not invalidate a settled CPU frame. A non-empty
    /// success does. A call with no outstanding buffer returns 0 and drops
    /// nothing.
    #[must_use]
    pub fn apply_commands(&mut self, ticket: u32, len: u32) -> u32 {
        if !self.ticket_is_current(ticket) {
            return 0;
        }
        let Some(bulk) = self.bulk.take() else {
            return 0;
        };
        let Ok(size) = usize::try_from(len) else {
            return 0;
        };
        if size != bulk.bytes.len() {
            return 0;
        }
        let previous_index = self.viewport.scroll().index();
        let applied = self
            .viewport
            .apply_commands(&bulk.bytes)
            .unwrap_or_default();
        if applied > 0 {
            if previous_index != self.viewport.scroll().index() {
                self.clear_selected_source();
            }
            self.note_command_changed();
        }
        applied
    }

    /// Drop the outstanding slot when `ticket` is current.
    ///
    /// A foreign ticket leaves the slot. Nothing is published and the last
    /// accepted frame stays. Ticket `0` is never current.
    #[must_use]
    pub fn release_bulk(&mut self, ticket: u32) -> bool {
        if !self.ticket_is_current(ticket) {
            return false;
        }
        self.bulk = None;
        true
    }

    const fn ticket_is_current(&self, ticket: u32) -> bool {
        match &self.bulk {
            Some(bulk) => ticket != 0 && ticket == bulk.ticket,
            None => false,
        }
    }

    fn fresh_ticket(&mut self) -> u32 {
        let ticket = self.next_ticket;
        self.next_ticket = next_nonzero(self.next_ticket);
        ticket
    }

    fn remember_frame(&mut self, bytes: &[u8]) {
        self.last_frame.clear();
        self.last_frame.extend_from_slice(bytes);
        self.stored_samples.clear();
        if !bytes.len().is_multiple_of(4) {
            return;
        }
        let mut offset = 0;
        while offset + 4 <= bytes.len() {
            let mut buf = [0_u8; 4];
            let Some(chunk) = bytes.get(offset..offset + 4) else {
                self.stored_samples.clear();
                return;
            };
            buf.copy_from_slice(chunk);
            self.stored_samples.push(Stored(f32::from_le_bytes(buf)));
            offset += 4;
        }
    }

    fn adopt_cpu_source(&mut self, requested: u32) {
        let Some(raster) = self.cpu.as_mut() else {
            self.viewport.set_generation(requested);
            return;
        };
        let next = if requested > raster.generation() {
            requested
        } else {
            raster.generation().saturating_add(1)
        };
        raster.note_new_source(next);
        self.viewport.set_generation(next);
        self.viewport.set_refining(raster.refining());
    }

    fn note_command_changed(&mut self) {
        let Some(raster) = self.cpu.as_mut() else {
            return;
        };
        let generation = raster.begin_interaction();
        self.viewport.set_refining(raster.refining());
        self.viewport.set_generation(generation);
    }

    /// UTF-8 JSON for the camera, the display area and the presentation.
    ///
    /// An empty vector is a refusal. [`Self::snapshot_error`] names it.
    /// Scroll, flags and generation are not in the record.
    #[must_use]
    pub fn snapshot(&mut self) -> Vec<u8> {
        match self.viewport.snapshot() {
            Ok(bytes) => {
                self.snapshot_code = 0;
                bytes
            }
            Err(error) => {
                self.snapshot_code = error.number();
                Vec::new()
            }
        }
    }

    /// The code from the last [`Self::snapshot`] call. Zero after a success.
    ///
    /// A restore does not change this. Its own return value is the restore code.
    #[must_use]
    pub const fn snapshot_error(&self) -> u32 {
        self.snapshot_code
    }

    /// Restore a snapshot. Zero is success. Any other value is
    /// [`ocelli_viewport::snapshot::SnapshotError::number`].
    ///
    /// A refusal leaves the viewport unchanged.
    #[must_use]
    pub fn restore_snapshot(&mut self, bytes: &[u8]) -> u32 {
        match self.viewport.restore(bytes) {
            Ok(()) => 0,
            Err(error) => error.number(),
        }
    }

    /// Copy the section 17.4 readback.
    ///
    /// Returns [`STATE_BYTES`] when `out` has that length, and 0 otherwise.
    #[must_use]
    pub fn copy_state(&self, out: &mut [u8]) -> u32 {
        if self.viewport.write_state(out).is_err() {
            return 0;
        }
        u32::try_from(STATE_BYTES).unwrap_or(0)
    }

    /// Copy the ring the same way [`EventRing::copy_pending`] does.
    #[must_use]
    pub fn copy_ring(&mut self, out: &mut [u8]) -> usize {
        self.ring.copy_pending(out)
    }

    /// Own the CPU frame buffer for this CSS box. The bytes stay in the session.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] or [`RasterError::Extent`] from [`CpuRaster::open`].
    pub fn bind_cpu(
        &mut self,
        css_width: f64,
        css_height: f64,
        dpr: f64,
    ) -> Result<[u32; 2], RasterError> {
        let raster = CpuRaster::open(css_width, css_height, dpr)?;
        let device = raster.device();
        let width = usize::try_from(device[0]).map_err(|_| RasterError::Extent)?;
        let height = usize::try_from(device[1]).map_err(|_| RasterError::Extent)?;
        let bytes = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RasterError::Extent)?;
        self.cpu_rgba = vec![0_u8; bytes];
        self.cpu_present = vec![0_u8; bytes];
        self.cpu = Some(raster);
        Ok(device)
    }

    /// Mark the CPU raster refining and advance its generation.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when [`Self::bind_cpu`] has not been called.
    pub fn begin_cpu_interaction(&mut self) -> Result<u32, RasterError> {
        let Some(raster) = self.cpu.as_mut() else {
            return Err(RasterError::Canvas);
        };
        let generation = raster.begin_interaction();
        self.viewport.set_refining(raster.refining());
        self.viewport.set_generation(generation);
        Ok(generation)
    }

    /// Draw one monochrome CPU frame into the session buffer.
    ///
    /// The presentation is borrowed. Kind 6 is not written here. The canvas
    /// path publishes after the bitmap is updated, and a native caller uses
    /// [`Self::publish_cpu_frame`] for the same boundary.
    ///
    /// # Errors
    ///
    /// [`RasterError`] from the raster. A stale completion does not change
    /// refining.
    pub fn draw_cpu_mono(
        &mut self,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        quality: RasterQuality,
    ) -> Result<RasterResult, RasterError> {
        let width = usize::try_from(target[0]).map_err(|_| RasterError::Extent)?;
        let height = usize::try_from(target[1]).map_err(|_| RasterError::Extent)?;
        let needed = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RasterError::Extent)?;
        let (outcome, refining, generation) = {
            let Self {
                viewport,
                cpu,
                cpu_rgba,
                ..
            } = self;
            let view = *viewport.view();
            let presentation = viewport.presentation();
            let Some(raster) = cpu.as_mut() else {
                return Err(RasterError::Canvas);
            };
            let Some(slot) = cpu_rgba.get_mut(..needed) else {
                return Err(RasterError::Extent);
            };
            let outcome = raster.raster_into(
                &view,
                presentation,
                stored,
                RasterGrid { source, target },
                quality,
                slot,
            )?;
            (outcome, raster.refining(), raster.generation())
        };
        if outcome.wrote {
            self.viewport.set_refining(refining);
            self.viewport.set_generation(generation);
            self.record_presentation(outcome.extent)?;
        }
        Ok(outcome)
    }

    /// Draw stored RGB through the raster's interleaved native resolution.
    ///
    /// The source length is accepted before [`CpuRaster::reserve_source`].
    /// The resolution itself is built inside that draw, before the pixel loop.
    /// Kind 6 is not written here.
    ///
    /// # Errors
    ///
    /// [`RasterError`] from the raster. [`RasterError::Unavailable`] when the
    /// RGB resolution itself is refused. A stale completion does not write.
    pub fn draw_cpu_rgb(
        &mut self,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        quality: RasterQuality,
    ) -> Result<RasterResult, RasterError> {
        let needed = rgba_len(target)?;
        let (outcome, refining, generation) = {
            let Self {
                viewport,
                cpu,
                cpu_rgba,
                ..
            } = self;
            let view = *viewport.view();
            let Some(raster) = cpu.as_mut() else {
                return Err(RasterError::Canvas);
            };
            CpuRaster::accept_rgb_source(stored, source)?;
            raster.reserve_source(source[0], source[1])?;
            let Some(slot) = cpu_rgba.get_mut(..needed) else {
                return Err(RasterError::Extent);
            };
            let outcome = raster.raster_rgb_into(
                &view,
                stored,
                RasterGrid { source, target },
                quality,
                slot,
            )?;
            (outcome, raster.refining(), raster.generation())
        };
        if outcome.wrote {
            self.viewport.set_refining(refining);
            self.viewport.set_generation(generation);
            self.record_presentation(outcome.extent)?;
        }
        Ok(outcome)
    }

    /// Publish kind 6 for a frame that was already drawn into the session.
    ///
    /// This does not read the pixels back to the caller.
    pub fn publish_cpu_frame(&mut self, extent: [u32; 2]) -> u64 {
        self.present(extent[0], extent[1])
    }

    /// Draw the last committed samples as monochrome into the owned CPU buffer.
    ///
    /// Kind 6 is not written here. The canvas path publishes only after a
    /// successful blit. The stored-sample vector is moved aside for the draw
    /// and put back. It is not cloned. Monochrome is remembered only after
    /// this draw returns Ok.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no frame has been committed or no raster
    /// is bound. Other [`RasterError`] values come from the draw. A refusal
    /// leaves the remembered shade unchanged.
    pub fn draw_committed_mono(&mut self) -> Result<RasterResult, RasterError> {
        let drawn = self.draw_requested(CpuShade::Mono)?;
        self.shade = CpuShade::Mono;
        Ok(drawn)
    }

    /// Draw the last committed samples as interleaved native RGB.
    ///
    /// RGB is remembered only after this draw returns Ok, including a valid
    /// idle of a compatible payload. Kind 6 is not written here.
    ///
    /// # Errors
    ///
    /// [`RasterError`] from the draw. A refusal leaves the remembered shade
    /// unchanged.
    pub fn draw_committed_rgb(&mut self) -> Result<RasterResult, RasterError> {
        let drawn = self.draw_requested(CpuShade::Rgb)?;
        self.shade = CpuShade::Rgb;
        Ok(drawn)
    }

    /// Accept photometric code 1 as the interleaved native RGB route.
    ///
    /// Code 0 and every other code return [`RasterError::Unavailable`].
    /// Code 1 does not remember RGB and does not draw. The draw commits the
    /// shade after the payload is accepted.
    ///
    /// # Errors
    ///
    /// [`RasterError::Unavailable`] for every code other than 1.
    pub fn select_supported_color(&mut self, photometric: u32) -> Result<(), RasterError> {
        if photometric != 1 {
            return Err(RasterError::Unavailable);
        }
        Ok(())
    }

    /// Draw the committed frame at the full device extent for the current generation.
    ///
    /// The shade is the one a successful [`Self::draw_committed_mono`] or
    /// [`Self::draw_committed_rgb`] remembered. This entry has no generation
    /// argument, so a caller cannot name a stale generation.
    ///
    /// # Errors
    ///
    /// [`RasterError`] from the draw.
    pub fn draw_settled_committed(&mut self) -> Result<RasterResult, RasterError> {
        let shade = self.shade;
        self.draw_requested(shade)
    }

    /// Begin an interaction and draw the remembered shade at the reduced extent.
    ///
    /// The reduced extent is `max(1, device / 2)` on each axis. The remembered
    /// payload is checked before the generation changes. Settlement is not
    /// recorded here.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no frame has been committed or no raster
    /// is bound. [`RasterError::Length`] when the remembered route does not
    /// match the committed samples. That refusal does not bump the generation.
    /// Other [`RasterError`] values come from the draw.
    pub fn draw_interaction(&mut self) -> Result<RasterResult, RasterError> {
        if self.last_meta.is_none() {
            return Err(RasterError::Canvas);
        }
        let target = self.interaction_target()?;
        self.accept_committed_layout()?;
        let shade = self.shade;
        self.begin_cpu_interaction()?;
        self.draw_committed_target(shade, RasterQuality::Interacting, target)
    }

    /// RGBA of the last CPU draw. The shell does not receive this slice.
    #[must_use]
    pub fn cpu_frame(&self) -> &[u8] {
        &self.cpu_rgba
    }

    /// Stored samples decoded from the last committed frame.
    #[must_use]
    pub fn stored_samples(&self) -> &[Stored] {
        &self.stored_samples
    }

    /// Device-sized bytes the canvas blits. The shell does not receive this slice.
    #[must_use]
    pub fn presented_frame(&self) -> &[u8] {
        &self.cpu_present
    }

    /// Device extent of the bound raster.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound.
    pub fn cpu_device(&self) -> Result<[u32; 2], RasterError> {
        self.cpu
            .as_ref()
            .map(CpuRaster::device)
            .ok_or(RasterError::Canvas)
    }

    /// Reduced interaction extent for the bound device.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound.
    /// [`RasterError::Extent`] when the device extent is empty.
    pub fn interaction_target(&self) -> Result<[u32; 2], RasterError> {
        reduced_extent(self.cpu_device()?)
    }

    /// Replace the CPU device pixel ratio and resize the owned frame buffer
    /// only after the raster accepts the scale.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound or the ratio is refused.
    /// [`RasterError::Extent`] when the new buffer does not fit.
    pub fn set_cpu_device_pixel_ratio(&mut self, dpr: f64) -> Result<[u32; 2], RasterError> {
        let Some(raster) = self.cpu.as_mut() else {
            return Err(RasterError::Canvas);
        };
        raster.set_device_pixel_ratio(dpr)?;
        self.sync_cpu_buffer()
    }

    /// Resize the bound CPU raster and replace the frame buffer when the
    /// device extent changes.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound or the size is refused.
    /// [`RasterError::Extent`] when the new buffer does not fit.
    pub fn resize_cpu(
        &mut self,
        css_width: f64,
        css_height: f64,
        dpr: f64,
    ) -> Result<[u32; 2], RasterError> {
        let Some(raster) = self.cpu.as_mut() else {
            return Err(RasterError::Canvas);
        };
        raster.resize(css_width, css_height, dpr)?;
        self.sync_cpu_buffer()
    }

    /// Keep refining set so a later draw writes the current frame again.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound.
    pub fn defer_cpu_presentation(&mut self) -> Result<(), RasterError> {
        let Some(raster) = self.cpu.as_mut() else {
            return Err(RasterError::Canvas);
        };
        raster.defer_presentation();
        self.viewport.set_refining(true);
        Ok(())
    }

    /// Record the current generation as presented.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when no raster is bound.
    pub fn accept_cpu_presentation(&mut self) -> Result<(), RasterError> {
        let Some(raster) = self.cpu.as_mut() else {
            return Err(RasterError::Canvas);
        };
        raster.accept_presentation();
        let generation = raster.generation();
        self.viewport.set_refining(false);
        self.viewport.set_generation(generation);
        Ok(())
    }

    fn cpu_generation(&self) -> Result<u32, RasterError> {
        self.cpu
            .as_ref()
            .map(CpuRaster::generation)
            .ok_or(RasterError::Canvas)
    }

    #[cfg(test)]
    fn cpu_allocations(&self) -> u32 {
        self.cpu.as_ref().map(CpuRaster::allocations).unwrap_or(0)
    }

    fn draw_requested(&mut self, shade: CpuShade) -> Result<RasterResult, RasterError> {
        let target = self.cpu_device()?;
        let generation = self.cpu_generation()?;
        self.draw_committed_target(shade, RasterQuality::Settled { generation }, target)
    }

    fn accept_committed_layout(&self) -> Result<(), RasterError> {
        let meta = self.last_meta.ok_or(RasterError::Canvas)?;
        let source = [meta.columns, meta.rows];
        match self.shade {
            CpuShade::Mono => CpuRaster::accept_mono_source(&self.stored_samples, source),
            CpuShade::Rgb => CpuRaster::accept_rgb_source(&self.stored_samples, source),
        }
    }

    fn draw_committed_target(
        &mut self,
        shade: CpuShade,
        quality: RasterQuality,
        target: [u32; 2],
    ) -> Result<RasterResult, RasterError> {
        let Some(meta) = self.last_meta else {
            return Err(RasterError::Canvas);
        };
        let source = [meta.columns, meta.rows];
        let stored = core::mem::take(&mut self.stored_samples);
        let drawn = match shade {
            CpuShade::Mono => self.draw_cpu_mono(&stored, source, target, quality),
            CpuShade::Rgb => self.draw_cpu_rgb(&stored, source, target, quality),
        };
        self.stored_samples = stored;
        drawn
    }

    /// Zero bytes past the drawn prefix, then replicate that prefix onto the
    /// device-sized presentation buffer. Replication is integer cell coverage.
    /// It is not a second sample of the source image.
    fn record_presentation(&mut self, drawn: [u32; 2]) -> Result<(), RasterError> {
        let device = self.cpu_device()?;
        let prefix = rgba_len(drawn)?;
        let full = rgba_len(device)?;
        if self.cpu_rgba.len() != full || self.cpu_present.len() != full || prefix > full {
            return Err(RasterError::OutputLength);
        }
        if let Some(tail) = self.cpu_rgba.get_mut(prefix..) {
            tail.fill(0);
        }
        replicate_cells(drawn, device, &self.cpu_rgba, &mut self.cpu_present)
    }

    fn sync_cpu_buffer(&mut self) -> Result<[u32; 2], RasterError> {
        let Some(raster) = self.cpu.as_ref() else {
            return Err(RasterError::Canvas);
        };
        let device = raster.device();
        let width = usize::try_from(device[0]).map_err(|_| RasterError::Extent)?;
        let height = usize::try_from(device[1]).map_err(|_| RasterError::Extent)?;
        let bytes = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RasterError::Extent)?;
        if self.cpu_rgba.len() != bytes {
            self.cpu_rgba = vec![0_u8; bytes];
        }
        if self.cpu_present.len() != bytes {
            self.cpu_present = vec![0_u8; bytes];
        }
        let refining = raster.refining();
        let generation = raster.generation();
        self.viewport.set_refining(refining);
        self.viewport.set_generation(generation);
        Ok(device)
    }

    /// Record a presented frame. The payload is generation, drawn width,
    /// drawn height and the viewport flags. The drawn extent is the reduced
    /// grid during interaction and the full device on settle. This does not
    /// read pixels back.
    pub fn present(&mut self, drawn_width: u32, drawn_height: u32) -> u64 {
        self.ring.push_presented(
            u32::from(self.viewport.id()),
            self.viewport.generation(),
            drawn_width,
            drawn_height,
            self.viewport.flags(),
        )
    }

    #[cfg(target_arch = "wasm32")]
    fn address_for(&self, bytes: &[u8]) -> u32 {
        let addr = match bytes.first() {
            Some(byte) => core::ptr::from_ref(byte).addr(),
            None => core::ptr::from_ref(empty_bulk()).addr(),
        };
        u32::try_from(addr).unwrap_or(0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn address_for(&mut self, bytes: &[u8]) -> u32 {
        let _ = bytes;
        let address = self.next_address;
        self.next_address = next_nonzero(self.next_address);
        address
    }
}

/// `max(1, device / 2)` on each axis. An empty axis is refused.
fn reduced_extent(device: [u32; 2]) -> Result<[u32; 2], RasterError> {
    if device[0] == 0 || device[1] == 0 {
        return Err(RasterError::Extent);
    }
    Ok([(device[0] / 2).max(1), (device[1] / 2).max(1)])
}

fn rgba_len(extent: [u32; 2]) -> Result<usize, RasterError> {
    let width = usize::try_from(extent[0]).map_err(|_| RasterError::Extent)?;
    let height = usize::try_from(extent[1]).map_err(|_| RasterError::Extent)?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RasterError::Extent)
}

fn rgba_index(x: u32, y: u32, columns: u32) -> Result<usize, RasterError> {
    let column = usize::try_from(x).map_err(|_| RasterError::Extent)?;
    let row = usize::try_from(y).map_err(|_| RasterError::Extent)?;
    let width = usize::try_from(columns).map_err(|_| RasterError::Extent)?;
    row.checked_mul(width)
        .and_then(|offset| offset.checked_add(column))
        .and_then(|index| index.checked_mul(4))
        .ok_or(RasterError::Extent)
}

fn cell_index(
    device_index: u32,
    device_extent: u32,
    drawn_extent: u32,
) -> Result<u32, RasterError> {
    if device_extent == 0 || drawn_extent == 0 {
        return Err(RasterError::Extent);
    }
    // The product is wider than a device index. A saturating `u32` multiply
    // folds the last cells of a wide presentation onto the previous cell.
    let product = u64::from(device_index)
        .checked_mul(u64::from(drawn_extent))
        .ok_or(RasterError::Extent)?;
    let mapped = product / u64::from(device_extent);
    let limit = u64::from(drawn_extent.saturating_sub(1));
    u32::try_from(mapped.min(limit)).map_err(|_| RasterError::Extent)
}

fn replicate_cells(
    drawn: [u32; 2],
    device: [u32; 2],
    source: &[u8],
    destination: &mut [u8],
) -> Result<(), RasterError> {
    let mut y = 0_u32;
    while y < device[1] {
        let source_y = cell_index(y, device[1], drawn[1])?;
        let mut x = 0_u32;
        while x < device[0] {
            let source_x = cell_index(x, device[0], drawn[0])?;
            let from = rgba_index(source_x, source_y, drawn[0])?;
            let to = rgba_index(x, y, device[0])?;
            let Some(src) = source.get(from..).and_then(|tail| tail.get(..4)) else {
                return Err(RasterError::OutputLength);
            };
            let Some(dst) = destination.get_mut(to..).and_then(|tail| tail.get_mut(..4)) else {
                return Err(RasterError::OutputLength);
            };
            dst.copy_from_slice(src);
            x = x.saturating_add(1);
        }
        y = y.saturating_add(1);
    }
    Ok(())
}

fn next_nonzero(current: u32) -> u32 {
    let next = current.wrapping_add(1);
    if next == 0 { 1 } else { next }
}

#[cfg(target_arch = "wasm32")]
fn empty_bulk() -> &'static u8 {
    static EMPTY_BULK: u8 = 0;
    &EMPTY_BULK
}

/// Wasm exports. Native tests call [`Session`] directly.
#[cfg(target_arch = "wasm32")]
pub(crate) mod bind {
    use std::cell::RefCell;

    use ocelli_viewport::boundary::{EVENT_HEADER_BYTES, EVENT_STRIDE};
    use ocelli_viewport::commands::{STATE_BYTES, StackViewport};
    use ocelli_viewport::ring::RING_CAPACITY;
    use wasm_bindgen::prelude::wasm_bindgen;

    use super::{FrameMeta, Session, SessionError};

    thread_local! {
        static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    }

    pub(crate) fn with_session<T>(body: impl FnOnce(&mut Session) -> T) -> Option<T> {
        SESSION.with(|slot| {
            let Ok(mut session) = slot.try_borrow_mut() else {
                return None;
            };
            if session.is_none() {
                let Ok(viewport) = StackViewport::try_identity(1) else {
                    return None;
                };
                *session = Some(Session::new(viewport));
            }
            session.as_mut().map(body)
        })
    }

    #[wasm_bindgen(js_name = alloc)]
    pub fn alloc(len: u32) -> u32 {
        with_session(|session| session.alloc(len)).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = bulk_ticket)]
    pub fn bulk_ticket() -> u32 {
        with_session(|session| session.bulk_ticket()).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = apply_commands)]
    pub fn apply_commands(ticket: u32, len: u32) -> u32 {
        with_session(|session| session.apply_commands(ticket, len)).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = commit_frame)]
    pub fn commit_frame(
        ticket: u32,
        len: u32,
        viewport: u32,
        columns: u32,
        rows: u32,
        generation: u32,
    ) -> u32 {
        let meta = FrameMeta {
            viewport,
            columns,
            rows,
            generation,
        };
        match with_session(|session| session.commit_frame(ticket, len, meta)) {
            Some(Ok(())) => 1,
            _ => 0,
        }
    }

    /// Drop the current bulk slot. A foreign ticket returns 0 and leaves it.
    #[wasm_bindgen(js_name = release_bulk)]
    pub fn release_bulk(ticket: u32) -> u32 {
        let released = with_session(|session| session.release_bulk(ticket)).unwrap_or(false);
        u32::from(released)
    }

    #[wasm_bindgen(js_name = copy_state)]
    pub fn copy_state() -> Vec<u8> {
        let mut out = vec![0_u8; STATE_BYTES];
        let copied = with_session(|session| session.copy_state(&mut out)).unwrap_or(0);
        if usize::try_from(copied).unwrap_or(0) == STATE_BYTES {
            out
        } else {
            Vec::new()
        }
    }

    #[wasm_bindgen(js_name = configure_stack)]
    pub fn configure_stack(ticket: u32, len: u32) -> u32 {
        with_session(|session| session.configure_stack(ticket, len).unwrap_or(0)).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = commit_selected_frame)]
    pub fn commit_selected_frame(
        ticket: u32,
        len: u32,
        viewport: u32,
        columns: u32,
        rows: u32,
        index: u32,
        epoch: u32,
    ) -> u32 {
        with_session(|session| {
            let meta = FrameMeta {
                viewport,
                columns,
                rows,
                generation: 0,
            };
            u32::from(
                session
                    .commit_selected_frame(ticket, len, meta, index, epoch)
                    .is_ok(),
            )
        })
        .unwrap_or(0)
    }

    /// Move the frame cache's budget. Evictions and the budget change leave
    /// through the ring.
    #[wasm_bindgen(js_name = set_frame_budget)]
    pub fn set_frame_budget(bytes: u32) -> u32 {
        with_session(|session| session.set_frame_budget(bytes)).unwrap_or(0)
    }

    /// Commit the selected frame and cache it under `content`. 1 committed,
    /// 0 refused as `commit_selected_frame` refuses.
    #[wasm_bindgen(js_name = commit_cached_frame)]
    pub fn commit_cached_frame(
        ticket: u32,
        len: u32,
        viewport: u32,
        columns: u32,
        rows: u32,
        index: u32,
        epoch: u32,
        content: u64,
    ) -> u32 {
        with_session(|session| {
            let meta = FrameMeta {
                viewport,
                columns,
                rows,
                generation: 0,
            };
            u32::from(
                session
                    .commit_cached_frame(ticket, len, meta, index, epoch, content)
                    .is_ok(),
            )
        })
        .unwrap_or(0)
    }

    /// 1 when the stack key at `index` answers for the identity and extent.
    /// A query is not a use.
    #[wasm_bindgen(js_name = frame_resident)]
    pub fn frame_resident(index: u32, epoch: u32, content: u64, columns: u32, rows: u32) -> u32 {
        u32::from(
            with_session(|session| session.frame_resident(index, epoch, content, columns, rows))
                .unwrap_or(false),
        )
    }

    /// 1 shown, 0 on a miss, a stale entry or a wrong epoch, and 2 when the
    /// index is not the selected one, which `showResident` answers with the
    /// coded `CommitRefused`.
    #[wasm_bindgen(js_name = show_resident)]
    pub fn show_resident(index: u32, epoch: u32, content: u64, columns: u32, rows: u32) -> u32 {
        with_session(
            |session| match session.show_resident(index, epoch, content, columns, rows) {
                Ok(shown) => shown,
                Err(SessionError::Selection) => 2,
                Err(_) => 0,
            },
        )
        .unwrap_or(0)
    }

    /// Release every cached frame of `series`. Returns the frame count.
    #[wasm_bindgen(js_name = release_series)]
    pub fn release_series(series: u64) -> u32 {
        with_session(|session| session.release_series(series)).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = request_frame_pressure)]
    pub fn request_frame_pressure() -> u32 {
        with_session(|session| session.request_frame_pressure()).unwrap_or(0)
    }

    /// The content id of the outstanding slot's bytes, or `undefined` on a
    /// refusal. `0` is a legitimate id, so `undefined` is the refusal.
    #[wasm_bindgen(js_name = content_id)]
    pub fn content_id(ticket: u32, len: u32, uid_len: u32, ts_len: u32) -> Option<u64> {
        with_session(|session| session.content_id(ticket, len, uid_len, ts_len)).unwrap_or(None)
    }

    /// Header plus pending events, already copied out of the ring.
    /// There is no `ring_ptr`. A pointer would be a view over linear memory.
    #[wasm_bindgen(js_name = drain_events)]
    pub fn drain_events() -> Vec<u8> {
        let capacity = usize::try_from(RING_CAPACITY).unwrap_or(0);
        let mut out =
            vec![0_u8; EVENT_HEADER_BYTES.saturating_add(capacity.saturating_mul(EVENT_STRIDE))];
        let copied = with_session(|session| session.copy_ring(&mut out)).unwrap_or(0);
        out.truncate(copied);
        out
    }

    #[wasm_bindgen(js_name = ring_capacity)]
    pub fn ring_capacity() -> u32 {
        RING_CAPACITY
    }

    /// Copied JSON. Empty when the session is missing or the record is refused.
    #[wasm_bindgen(js_name = snapshot)]
    pub fn snapshot() -> Vec<u8> {
        with_session(super::Session::snapshot).unwrap_or_default()
    }

    /// Code from the last [`snapshot`]. Zero after a success and when no
    /// session could be opened.
    #[wasm_bindgen(js_name = snapshot_error)]
    pub fn snapshot_error() -> u32 {
        with_session(|session| session.snapshot_error()).unwrap_or(0)
    }

    /// Zero on success. A missing session is
    /// [`ocelli_viewport::snapshot::SNAPSHOT_ERROR_CHAIN_UNAVAILABLE`],
    /// which is not success.
    #[wasm_bindgen(js_name = restore_snapshot)]
    pub fn restore_snapshot(bytes: &[u8]) -> u32 {
        with_session(|session| session.restore_snapshot(bytes))
            .unwrap_or(ocelli_viewport::snapshot::SNAPSHOT_ERROR_CHAIN_UNAVAILABLE)
    }
}

#[cfg(test)]
mod tests {
    use ocelli_cache::{FrameKey, SeriesId};
    use ocelli_core::Stored;
    use ocelli_pixel::{
        LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation,
        PresentationLutEvidence, VoiFunction, VoiTransform, lut_descriptor_clones,
    };
    use ocelli_render::{RasterError, RasterQuality};
    use ocelli_viewport::commands::StackViewport;
    use ocelli_viewport::presentation::{Colormap, Presentation};
    use ocelli_viewport::ring::KIND_PRESENTED;
    use ocelli_viewport::scroll::StackSequence;

    use super::{FrameMeta, Session, SessionError};

    use std::sync::Mutex;

    /// `lut_descriptor_clones` is a process-wide counter and the test binary
    /// runs its tests on separate threads. Every test that can clone a
    /// presentation takes this lock, so `draw_and_idle_do_not_clone_lut_buffers`
    /// measures only its own draw and cannot see a sibling's legitimate clone.
    static CLONE_LOCK: Mutex<()> = Mutex::new(());

    fn clone_guard() -> std::sync::MutexGuard<'static, ()> {
        CLONE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn require_viewport(id: u16) -> StackViewport {
        let built = StackViewport::try_identity(id);
        assert!(built.is_ok());
        match built {
            Ok(value) => value,
            Err(_) => unreachable!("identity chain is in domain"),
        }
    }

    fn fit_bytes(id: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&24_u32.to_le_bytes());
        out.extend_from_slice(&8.0_f64.to_le_bytes());
        out.extend_from_slice(&8.0_f64.to_le_bytes());
        out.extend_from_slice(&2_u32.to_le_bytes());
        out.extend_from_slice(&4_u32.to_le_bytes());
        out
    }

    fn fill(session: &mut Session, address: u32, bytes: &[u8]) {
        let Some(slot) = session.bulk_mut(address) else {
            unreachable!("address was just allocated");
        };
        assert_eq!(slot.len(), bytes.len());
        slot.copy_from_slice(bytes);
    }

    fn slot(session: &mut Session, len: u32) -> (u32, u32) {
        let address = session.alloc(len);
        (address, session.bulk_ticket())
    }

    #[test]
    fn alloc_commit_and_a_duplicate_commit_keeps_the_accepted_bytes() {
        let mut session = Session::new(require_viewport(7));
        assert_eq!(session.alloc(67_108_865), 0);
        let (held, ticket) = slot(&mut session, 4);
        assert_ne!(held, 0);
        assert_ne!(ticket, 0);
        assert_ne!(held, ticket);
        assert_eq!(session.alloc(4), 0);
        fill(&mut session, held, &[9, 8, 7, 6]);
        let meta = FrameMeta {
            viewport: 7,
            columns: 2,
            rows: 4,
            generation: 3,
        };
        assert_eq!(session.commit_frame(ticket, 4, meta), Ok(()));
        assert_eq!(session.last_frame(), &[9, 8, 7, 6]);
        assert_eq!(session.last_meta(), Some(meta));
        assert_eq!(session.viewport().generation(), 3);
        assert_eq!(
            session.commit_frame(ticket, 4, meta),
            Err(SessionError::Stale)
        );
        assert_eq!(session.last_frame(), &[9, 8, 7, 6]);
        let (again, again_ticket) = slot(&mut session, 4);
        assert_ne!(again, 0);
        assert_ne!(again, held);
        assert_ne!(again_ticket, ticket);
        assert_eq!(
            session.commit_frame(ticket, 4, meta),
            Err(SessionError::Stale)
        );
        assert_eq!(session.bulk_ticket(), again_ticket);
        assert_eq!(session.commit_frame(again_ticket, 4, meta), Ok(()));
    }

    #[test]
    fn a_mismatched_commit_releases_the_slot_and_keeps_no_bytes() {
        let mut session = Session::new(require_viewport(7));
        let (address, ticket) = slot(&mut session, 4);
        fill(&mut session, address, &[1, 2, 3, 4]);
        let meta = FrameMeta {
            viewport: 7,
            columns: 1,
            rows: 1,
            generation: 1,
        };
        assert_eq!(
            session.commit_frame(ticket, 3, meta),
            Err(SessionError::Length)
        );
        assert!(session.last_frame().is_empty());
        assert_eq!(session.last_meta(), None);
        let foreign = FrameMeta {
            viewport: 9,
            columns: 1,
            rows: 1,
            generation: 1,
        };
        let (next, next_ticket) = slot(&mut session, 4);
        assert_ne!(next, 0);
        fill(&mut session, next, &[1, 2, 3, 4]);
        assert_eq!(
            session.commit_frame(next_ticket, 4, foreign),
            Err(SessionError::Viewport)
        );
        assert!(session.last_frame().is_empty());
        assert_ne!(session.alloc(1), 0);
    }

    #[test]
    fn an_empty_alloc_is_a_nonzero_token_and_zero_is_still_failure() {
        let mut session = Session::new(require_viewport(7));
        let (address, ticket) = slot(&mut session, 0);
        assert_ne!(address, 0);
        assert_ne!(ticket, 0);
        let meta = FrameMeta {
            viewport: 7,
            columns: 0,
            rows: 0,
            generation: 4,
        };
        assert_eq!(session.commit_frame(ticket, 0, meta), Ok(()));
        assert!(session.last_frame().is_empty());
        assert_eq!(session.viewport().generation(), 4);
        assert_eq!(session.commit_frame(0, 0, meta), Err(SessionError::Stale));
    }

    #[test]
    fn apply_commands_uses_the_slot_once_and_a_refusal_returns_zero() {
        let _clone_guard = clone_guard();
        let viewport = require_viewport(7);
        let mut session = Session::new(viewport);
        let bytes = fit_bytes(7);
        let (address, ticket) = slot(&mut session, u32::try_from(bytes.len()).unwrap_or(0));
        assert_ne!(address, 0);
        fill(&mut session, address, &bytes);
        assert_eq!(
            session.apply_commands(ticket, u32::try_from(bytes.len()).unwrap_or(0)),
            1
        );
        assert_eq!(session.viewport().columns(), 2);
        assert!(session.bulk_mut(address).is_none());

        let mut refused = fit_bytes(7);
        refused.extend_from_slice(&7_u16.to_le_bytes());
        refused.extend_from_slice(&7_u16.to_le_bytes());
        refused.extend_from_slice(&17_u32.to_le_bytes());
        refused.extend_from_slice(&40.0_f64.to_le_bytes());
        refused.extend_from_slice(&0.0_f64.to_le_bytes());
        refused.push(0);
        let before = session.viewport().columns();
        let (bad, bad_ticket) = slot(&mut session, u32::try_from(refused.len()).unwrap_or(0));
        fill(&mut session, bad, &refused);
        assert_eq!(
            session.apply_commands(0, u32::try_from(refused.len()).unwrap_or(0)),
            0
        );
        assert_eq!(session.bulk_ticket(), bad_ticket);
        assert_eq!(
            session.apply_commands(bad_ticket, u32::try_from(refused.len()).unwrap_or(0)),
            0
        );
        assert_eq!(session.viewport().columns(), before);
        assert_ne!(session.alloc(1), 0);
    }

    #[test]
    fn a_voi_sequence_batch_returns_zero_and_the_fit_does_not_stick() {
        let _clone_guard = clone_guard();
        let table = LutDescriptor::new(3, 0, 8, vec![5.0, 15.0, 25.0]);
        assert!(table.is_ok());
        let Ok(table) = table else {
            unreachable!("three-entry descriptor is in domain");
        };
        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
        assert!(modality.is_ok());
        let Ok(modality) = modality else {
            unreachable!("slope 1 is in domain");
        };
        let voi = VoiTransform::new(
            Some(table),
            &[40.0],
            &[400.0],
            0,
            VoiFunction::Linear,
            0.0,
            255.0,
        );
        assert!(voi.is_ok());
        let Ok(voi) = voi else {
            unreachable!("sequence window is in domain");
        };
        let chain = LutChain::new(
            modality,
            voi,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        );
        assert!(chain.is_ok());
        let Ok(chain) = chain else {
            unreachable!("monochrome chain is in domain");
        };
        let presentation = Presentation::new(
            chain,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
            Colormap::Grey,
        );
        let keys = [FrameKey::new(SeriesId(1), 0, 0)];
        let viewport = StackViewport::new(7, presentation, StackSequence::new(&keys));
        let mut session = Session::new(viewport);
        let mut bytes = fit_bytes(7);
        bytes.extend_from_slice(&7_u16.to_le_bytes());
        bytes.extend_from_slice(&7_u16.to_le_bytes());
        bytes.extend_from_slice(&17_u32.to_le_bytes());
        bytes.extend_from_slice(&40.0_f64.to_le_bytes());
        bytes.extend_from_slice(&400.0_f64.to_le_bytes());
        bytes.push(0);
        let (address, ticket) = slot(&mut session, u32::try_from(bytes.len()).unwrap_or(0));
        fill(&mut session, address, &bytes);
        assert_eq!(
            session.apply_commands(ticket, u32::try_from(bytes.len()).unwrap_or(0)),
            0
        );
        assert_eq!(session.viewport().columns(), 0);
    }

    fn read_u32(bytes: &[u8], at: usize) -> u32 {
        let mut buf = [0_u8; 4];
        let Some(slot) = bytes.get(at..at + 4) else {
            return u32::MAX;
        };
        buf.copy_from_slice(slot);
        u32::from_le_bytes(buf)
    }

    #[test]
    fn the_bulk_limit_is_64_mib_inclusive() {
        let mut session = Session::new(require_viewport(1));
        let (address, ticket) = slot(&mut session, 67_108_864);
        assert_ne!(address, 0);
        assert_ne!(ticket, 0);
        assert_eq!(
            session.bulk_mut(address).map(|bytes| bytes.len()),
            Some(67_108_864)
        );
    }

    #[test]
    fn state_and_presented_events_are_copies() {
        let mut session = Session::new(require_viewport(7));
        assert_eq!(session.copy_state(&mut [0_u8; 4]), 0);
        let mut state = [0_u8; 96];
        assert_eq!(session.copy_state(&mut state), 96);
        let (_address, ticket) = slot(&mut session, 0);
        let meta = FrameMeta {
            viewport: 7,
            columns: 0,
            rows: 0,
            generation: 9,
        };
        assert_eq!(session.commit_frame(ticket, 0, meta), Ok(()));
        assert_eq!(session.present(20, 10), 0);
        let mut out = vec![0_u8; 16 + 48];
        assert_eq!(session.copy_ring(&mut out), 16 + 48);
        assert_eq!(read_u32(&out, 0), 1);
        assert_eq!(read_u32(&out, 16), KIND_PRESENTED);
        assert_eq!(read_u32(&out, 32), 9);
        assert_eq!(read_u32(&out, 36), 20);
        assert_eq!(read_u32(&out, 40), 10);
        assert_eq!(session.copy_ring(&mut out), 16);
        assert_eq!(read_u32(&out, 0), 0);
    }

    #[test]
    fn a_foreign_commit_keeps_the_outstanding_bytes() {
        let mut session = Session::new(require_viewport(7));
        let (held, ticket) = slot(&mut session, 4);
        assert_ne!(held, 0);
        fill(&mut session, held, &[4, 3, 2, 1]);
        let meta = FrameMeta {
            viewport: 7,
            columns: 1,
            rows: 1,
            generation: 2,
        };
        assert_eq!(session.commit_frame(0, 4, meta), Err(SessionError::Stale));
        assert_eq!(session.bulk_ticket(), ticket);
        assert_eq!(session.commit_frame(ticket, 4, meta), Ok(()));
        assert_eq!(session.last_frame(), &[4, 3, 2, 1]);
    }

    #[test]
    fn draw_and_idle_do_not_clone_lut_buffers() {
        let _clone_guard = clone_guard();
        let table = LutDescriptor::new(3, 0, 8, vec![5.0, 15.0, 25.0]);
        assert!(table.is_ok());
        let Ok(table) = table else {
            unreachable!("three-entry descriptor is in domain");
        };
        let modality_table = LutDescriptor::new(2, 0, 8, vec![1.0, 2.0]);
        assert!(modality_table.is_ok());
        let Ok(modality_table) = modality_table else {
            unreachable!("two-entry descriptor is in domain");
        };
        let modality = ModalityTransform::new(Some(modality_table), Some(1.0), Some(0.0));
        assert!(modality.is_ok());
        let Ok(modality) = modality else {
            unreachable!("modality sequence is in domain");
        };
        let voi = VoiTransform::new(
            Some(table),
            &[40.0],
            &[400.0],
            0,
            VoiFunction::Linear,
            0.0,
            255.0,
        );
        assert!(voi.is_ok());
        let Ok(voi) = voi else {
            unreachable!("sequence window is in domain");
        };
        let chain = LutChain::new(
            modality,
            voi,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        );
        assert!(chain.is_ok());
        let Ok(chain) = chain else {
            unreachable!("monochrome chain is in domain");
        };
        let presentation = Presentation::new(
            chain,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
            Colormap::Grey,
        );
        let viewport = StackViewport::new(7, presentation, StackSequence::new(&[]));
        let mut session = Session::new(viewport);
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        let before = lut_descriptor_clones();
        let drawn = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(drawn.is_ok());
        assert_eq!(lut_descriptor_clones(), before);
        let idle = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(idle.is_ok());
        assert_eq!(lut_descriptor_clones(), before);
    }

    #[test]
    fn cpu_draw_publishes_kind_6_and_a_stale_settle_does_not() {
        let mut session = Session::new(require_viewport(7));
        let device = session.bind_cpu(1.0, 1.0, 1.0);
        assert!(device.is_ok());
        let settled = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(settled.is_ok());
        let mut out = vec![0_u8; 16 + 48];
        assert_eq!(session.copy_ring(&mut out), 16);
        let Ok(settled) = settled else {
            unreachable!("the settled draw was asserted");
        };
        assert_ne!(session.publish_cpu_frame(settled.extent), u64::MAX);
        assert_eq!(session.copy_ring(&mut out), 16 + 48);
        assert_eq!(read_u32(&out, 16), KIND_PRESENTED);
        assert_eq!(read_u32(&out, 44), 0);
        assert_eq!(session.begin_cpu_interaction(), Ok(1));
        let interacting =
            session.draw_cpu_mono(&[Stored(40.0)], [1, 1], [1, 1], RasterQuality::Interacting);
        assert!(interacting.is_ok());
        let Ok(interacting) = interacting else {
            unreachable!("the interacting draw was asserted");
        };
        assert_ne!(session.publish_cpu_frame(interacting.extent), u64::MAX);
        assert_eq!(session.copy_ring(&mut out), 16 + 48);
        assert_eq!(read_u32(&out, 44) & 8, 8);
        let stale = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(matches!(stale, Err(RasterError::Stale)));
        assert_eq!(session.copy_ring(&mut out), 16);
        assert_eq!(session.viewport().flags() & 8, 8);
    }

    #[test]
    fn a_reduced_presentation_replicates_the_drawn_prefix_across_the_device() {
        let mut session = Session::new(require_viewport(7));
        assert_eq!(session.bind_cpu(4.0, 2.0, 1.0), Ok([4, 2]));
        let mut bytes = Vec::new();
        for step in 1_u16..=24 {
            bytes.extend_from_slice(&(f32::from(step) * 10.0).to_le_bytes());
        }
        let (address, ticket) = slot(&mut session, 96);
        fill(&mut session, address, &bytes);
        assert_eq!(
            session.commit_frame(
                ticket,
                96,
                FrameMeta {
                    viewport: 7,
                    columns: 4,
                    rows: 2,
                    generation: 1,
                },
            ),
            Ok(())
        );
        let seeded = session.draw_committed_rgb();
        assert!(matches!(seeded, Ok(result) if result.wrote && result.extent == [4, 2]));
        let before = session.presented_frame().to_vec();
        assert_eq!(before.len(), 32);
        // The seeded frame must be nonuniform on both axes, or a replication
        // that stops short would leave a tail that already equals the prefix.
        let mut cells = before.chunks_exact(4);
        let first = cells.next();
        let second = cells.next();
        let third = cells.next();
        let fourth = cells.next();
        let fifth = cells.next();
        let sixth = cells.next();
        assert!(first.is_some() && second.is_some() && third.is_some() && fourth.is_some());
        assert!(fifth.is_some() && sixth.is_some());
        assert_ne!(first, second);
        assert_ne!(third, fourth);
        assert_ne!(fifth, sixth);

        assert!(session.begin_cpu_interaction().is_ok());
        let reduced = session.draw_cpu_mono(
            &[
                Stored(10.0),
                Stored(20.0),
                Stored(30.0),
                Stored(40.0),
                Stored(50.0),
                Stored(60.0),
                Stored(70.0),
                Stored(80.0),
            ],
            [4, 2],
            [1, 1],
            RasterQuality::Interacting,
        );
        assert!(matches!(reduced, Ok(result) if result.wrote && result.extent == [1, 1]));
        let frame = session.cpu_frame();
        let Some(prefix) = frame.get(..4) else {
            unreachable!("the reduced draw wrote four bytes");
        };
        let presented = session.presented_frame();
        assert_eq!(presented.len(), 32);
        for cell in presented.chunks_exact(4) {
            assert_eq!(cell, prefix, "each device cell is the drawn prefix");
        }

        // Kind 6 carries the drawn extent, not the device extent.
        let mut out = vec![0_u8; 16 + 48];
        assert_ne!(session.publish_cpu_frame([1, 1]), u64::MAX);
        assert_eq!(session.copy_ring(&mut out), 16 + 48);
        assert_eq!(read_u32(&out, 16), KIND_PRESENTED);
        assert_eq!(read_u32(&out, 36), 1);
        assert_eq!(read_u32(&out, 40), 1);
        assert_eq!(session.cpu_device(), Ok([4, 2]));
    }

    #[test]
    fn a_committed_frame_draws_without_publishing_kind_6() {
        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        let sample = 40.0_f32.to_le_bytes();
        let (address, ticket) = slot(&mut session, 4);
        fill(&mut session, address, &sample);
        assert_eq!(
            session.commit_frame(
                ticket,
                4,
                FrameMeta {
                    viewport: 7,
                    columns: 1,
                    rows: 1,
                    generation: 1,
                },
            ),
            Ok(())
        );
        let drawn = session.draw_committed_mono();
        assert!(matches!(drawn, Ok(result) if result.wrote && result.extent == [1, 1]));
        let mut out = vec![0_u8; 16 + 48];
        assert_eq!(session.copy_ring(&mut out), 16);
        let Ok(drawn) = drawn else {
            unreachable!("the committed draw was asserted");
        };
        assert_ne!(session.publish_cpu_frame(drawn.extent), u64::MAX);
        assert_eq!(session.copy_ring(&mut out), 16 + 48);
        assert_eq!(read_u32(&out, 16), KIND_PRESENTED);
        let idle = session.draw_committed_mono();
        assert!(matches!(idle, Ok(result) if !result.wrote));
        assert_eq!(session.copy_ring(&mut out), 16);
    }

    #[test]
    fn release_bulk_drops_only_the_current_ticket() {
        let mut session = Session::new(require_viewport(7));
        let (address, ticket) = slot(&mut session, 4);
        fill(&mut session, address, &[1, 2, 3, 4]);
        assert!(!session.release_bulk(0));
        assert!(!session.release_bulk(ticket.wrapping_add(1)));
        assert_eq!(session.bulk_ticket(), ticket);
        assert!(session.bulk_mut(address).is_some());
        assert!(session.release_bulk(ticket));
        assert_eq!(session.bulk_ticket(), 0);
        assert!(session.bulk_mut(address).is_none());
        let (_again, again_ticket) = slot(&mut session, 4);
        assert_ne!(again_ticket, ticket);
        assert!(session.release_bulk(again_ticket));
        assert_eq!(session.bulk_ticket(), 0);
    }

    #[test]
    fn a_deferred_presentation_stays_retryable_until_it_is_accepted() {
        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        let sample = 40.0_f32.to_le_bytes();
        let (address, ticket) = slot(&mut session, 4);
        fill(&mut session, address, &sample);
        assert_eq!(
            session.commit_frame(
                ticket,
                4,
                FrameMeta {
                    viewport: 7,
                    columns: 1,
                    rows: 1,
                    generation: 1,
                },
            ),
            Ok(())
        );
        let drawn = session.draw_committed_mono();
        assert!(matches!(drawn, Ok(result) if result.wrote && result.extent == [1, 1]));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.viewport().flags() & 8, 0);
        assert!(session.defer_cpu_presentation().is_ok());
        assert_eq!(session.viewport().flags() & 8, 8);
        let retry = session.draw_committed_mono();
        assert!(matches!(retry, Ok(result) if result.wrote));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert!(session.defer_cpu_presentation().is_ok());
        assert!(session.accept_cpu_presentation().is_ok());
        assert_eq!(session.viewport().flags() & 8, 0);
        let idle = session.draw_committed_mono();
        assert!(matches!(idle, Ok(result) if !result.wrote));
        let mut out = vec![0_u8; 16 + 48];
        assert_eq!(session.copy_ring(&mut out), 16);
    }

    #[test]
    fn committed_rgb_replicates_the_reduced_sample_and_settles_latest() {
        let _clone_guard = clone_guard();
        // Identity view, source 2 by 1, device 2 by 1, dpr 1.
        // Unfitted origin is 0.5 CSS pixels and the raster adds that half
        // back, so the image coordinate equals CSS. Full columns sample
        // 0.5 and 1.5, which are stored columns 0 and 1. The reduced centre
        // samples CSS 1.0, which is stored column 1. Exact channel values
        // in 0..=255 stay those bytes. Alpha is 255.
        assert_eq!(super::reduced_extent([2, 1]), Ok([1, 1]));
        assert_eq!(super::reduced_extent([1, 1]), Ok([1, 1]));
        assert_eq!(super::reduced_extent([3, 4]), Ok([1, 2]));
        assert!(matches!(
            super::reduced_extent([0, 1]),
            Err(RasterError::Extent)
        ));

        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(2.0, 1.0, 1.0).is_ok());
        assert_eq!(session.cpu_device(), Ok([2, 1]));
        assert_eq!(session.interaction_target(), Ok([1, 1]));
        let mut bytes = Vec::new();
        for value in [10.0_f32, 20.0, 30.0, 40.0, 50.0, 60.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let (address, ticket) = slot(&mut session, 24);
        fill(&mut session, address, &bytes);
        assert_eq!(
            session.commit_frame(
                ticket,
                24,
                FrameMeta {
                    viewport: 7,
                    columns: 2,
                    rows: 1,
                    generation: 1,
                },
            ),
            Ok(())
        );
        assert_eq!(session.viewport().generation(), 1);
        assert!(matches!(
            session.select_supported_color(0),
            Err(RasterError::Unavailable)
        ));
        assert!(matches!(
            session.select_supported_color(2),
            Err(RasterError::Unavailable)
        ));
        assert_eq!(session.cpu_allocations(), 0);
        assert_eq!(session.cpu_frame(), &[0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(session.select_supported_color(1).is_ok());
        assert_eq!(session.cpu_allocations(), 0);

        let full = [10, 20, 30, 255, 40, 50, 60, 255];
        let settled = session.draw_committed_rgb();
        assert!(matches!(
            settled,
            Ok(result) if result.wrote && result.extent == [2, 1] && result.generation == 1 && !result.refining
        ));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert_eq!(session.viewport().flags() & 8, 0);
        let allocations = session.cpu_allocations();
        // Nearest colour reserve grows the RGB scratch only.
        assert_eq!(allocations, 1);
        let present_capacity = session.cpu_present.capacity();

        let rejected = session.draw_committed_mono();
        assert!(matches!(rejected, Err(RasterError::Length)));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert_eq!(session.viewport().generation(), 1);
        assert_eq!(session.viewport().flags() & 8, 0);
        assert_eq!(session.cpu_allocations(), allocations);

        let reduced = session.draw_interaction();
        assert!(matches!(
            reduced,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 2 && result.refining
        ));
        assert_eq!(session.cpu_frame(), &[40, 50, 60, 255, 0, 0, 0, 0]);
        assert_eq!(
            session.presented_frame(),
            &[40, 50, 60, 255, 40, 50, 60, 255]
        );
        assert_eq!(session.viewport().flags() & 8, 8);
        assert_eq!(session.viewport().generation(), 2);
        assert_eq!(session.cpu_allocations(), allocations);
        assert_eq!(session.cpu_present.capacity(), present_capacity);

        let stale = session.draw_cpu_rgb(
            &[
                Stored(10.0),
                Stored(20.0),
                Stored(30.0),
                Stored(40.0),
                Stored(50.0),
                Stored(60.0),
            ],
            [2, 1],
            [2, 1],
            RasterQuality::Settled { generation: 1 },
        );
        assert!(matches!(stale, Err(RasterError::Stale)));
        assert_eq!(
            session.presented_frame(),
            &[40, 50, 60, 255, 40, 50, 60, 255]
        );
        assert_eq!(session.cpu_frame(), &[40, 50, 60, 255, 0, 0, 0, 0]);

        let latest = session.draw_settled_committed();
        assert!(matches!(
            latest,
            Ok(result) if result.wrote && result.extent == [2, 1] && result.generation == 2
        ));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert!(session.defer_cpu_presentation().is_ok());
        assert_eq!(session.viewport().flags() & 8, 8);
        assert!(session.accept_cpu_presentation().is_ok());
        assert_eq!(session.viewport().flags() & 8, 0);
        let idle = session.draw_settled_committed();
        assert!(matches!(
            idle,
            Ok(result) if !result.wrote && result.generation == 2 && result.extent == [0, 0]
        ));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert_eq!(session.cpu_allocations(), allocations);
        assert_eq!(session.cpu_present.capacity(), present_capacity);
        assert!(matches!(
            session.select_supported_color(0),
            Err(RasterError::Unavailable)
        ));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);

        invert_committed(&mut session);
        assert_eq!(session.viewport().generation(), 3);
        assert_eq!(session.viewport().flags() & 9, 9);
        assert_eq!(session.cpu_frame(), &full);
        let rejected_again = session.draw_committed_mono();
        assert!(matches!(rejected_again, Err(RasterError::Length)));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert_eq!(session.viewport().generation(), 3);
        assert_eq!(session.viewport().flags() & 9, 9);
        let after_invalidation = session.draw_interaction();
        assert!(matches!(
            after_invalidation,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 4 && result.refining
        ));
        assert_eq!(
            session.presented_frame(),
            &[40, 50, 60, 255, 40, 50, 60, 255]
        );
        assert_eq!(session.viewport().generation(), 4);
        let restored = session.draw_committed_rgb();
        assert!(matches!(
            restored,
            Ok(result) if result.wrote && result.extent == [2, 1] && result.generation == 4 && !result.refining
        ));
        assert_eq!(session.cpu_frame(), &full);
        assert_eq!(session.presented_frame(), &full);
        assert_eq!(session.viewport().flags() & 8, 0);
        assert_eq!(session.viewport().flags() & 1, 1);
        assert_eq!(session.cpu_allocations(), allocations);
    }

    #[test]
    fn a_rejected_colour_request_keeps_the_monochrome_route() {
        let _clone_guard = clone_guard();
        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        commit_stored(&mut session, &[40.0], 1, 1, 1);
        let settled = session.draw_committed_mono();
        assert!(matches!(
            settled,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 1 && !result.refining
        ));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.cpu_allocations(), 0);

        let rejected = session.draw_committed_rgb();
        assert!(matches!(rejected, Err(RasterError::Length)));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.viewport().generation(), 1);
        assert_eq!(session.viewport().flags() & 8, 0);
        assert_eq!(session.cpu_allocations(), 0);

        let interaction = session.draw_interaction();
        assert!(matches!(
            interaction,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 2 && result.refining
        ));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.viewport().flags() & 8, 8);
        let recovered = session.draw_committed_mono();
        assert!(matches!(
            recovered,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 2 && !result.refining
        ));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.viewport().flags() & 8, 0);

        let mut invalidated = Session::new(require_viewport(7));
        assert!(invalidated.bind_cpu(1.0, 1.0, 1.0).is_ok());
        commit_stored(&mut invalidated, &[40.0], 1, 1, 1);
        assert!(invalidated.draw_committed_mono().is_ok());
        invert_committed(&mut invalidated);
        assert_eq!(invalidated.viewport().generation(), 2);
        assert_eq!(invalidated.viewport().flags() & 9, 9);
        assert_eq!(invalidated.cpu_frame(), &[255, 255, 255, 255]);
        let refused = invalidated.draw_committed_rgb();
        assert!(matches!(refused, Err(RasterError::Length)));
        assert_eq!(invalidated.viewport().generation(), 2);
        assert_eq!(invalidated.viewport().flags() & 9, 9);
        assert_eq!(invalidated.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(invalidated.cpu_allocations(), 0);
        let drawn = invalidated.draw_interaction();
        assert!(matches!(
            drawn,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 3 && result.refining
        ));
        assert_eq!(invalidated.cpu_frame(), &[0, 0, 0, 255]);
        assert_eq!(invalidated.viewport().flags() & 9, 9);
        let settled_again = invalidated.draw_committed_mono();
        assert!(matches!(
            settled_again,
            Ok(result) if result.wrote && result.generation == 3 && !result.refining
        ));
        assert_eq!(invalidated.cpu_frame(), &[0, 0, 0, 255]);
        assert_eq!(invalidated.viewport().flags() & 1, 1);
        assert_eq!(invalidated.viewport().flags() & 8, 0);
    }

    #[test]
    fn an_incompatible_interaction_does_not_bump_the_generation() {
        let _clone_guard = clone_guard();
        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        commit_stored(&mut session, &[10.0, 20.0, 30.0], 1, 1, 1);
        let settled = session.draw_committed_rgb();
        assert!(matches!(settled, Ok(result) if result.wrote && result.generation == 1));
        assert_eq!(session.cpu_frame(), &[10, 20, 30, 255]);
        let allocations = session.cpu_allocations();
        // Nearest colour reserve grows the RGB scratch only.
        assert_eq!(allocations, 1);
        commit_stored(&mut session, &[40.0], 1, 1, 1);
        assert_eq!(session.viewport().generation(), 2);
        assert_eq!(session.viewport().flags() & 8, 8);
        assert_eq!(session.cpu_frame(), &[10, 20, 30, 255]);
        let refused = session.draw_interaction();
        assert!(matches!(refused, Err(RasterError::Length)));
        assert_eq!(session.viewport().generation(), 2);
        assert_eq!(session.viewport().flags() & 8, 8);
        assert_eq!(session.cpu_frame(), &[10, 20, 30, 255]);
        assert_eq!(session.cpu_allocations(), allocations);
        let recovered = session.draw_committed_mono();
        assert!(matches!(
            recovered,
            Ok(result) if result.wrote && result.extent == [1, 1] && result.generation == 2 && !result.refining
        ));
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        assert_eq!(session.viewport().flags() & 8, 0);
    }

    fn commit_stored(
        session: &mut Session,
        samples: &[f32],
        columns: u32,
        rows: u32,
        generation: u32,
    ) {
        let mut bytes = Vec::new();
        for value in samples {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let len = u32::try_from(bytes.len()).unwrap_or(0);
        let (address, ticket) = slot(session, len);
        fill(session, address, &bytes);
        assert_eq!(
            session.commit_frame(
                ticket,
                len,
                FrameMeta {
                    viewport: 7,
                    columns,
                    rows,
                    generation,
                },
            ),
            Ok(())
        );
    }

    fn invert_committed(session: &mut Session) {
        let bytes = command_bytes(9, &[1]);
        let len = u32::try_from(bytes.len()).unwrap_or(0);
        let (address, ticket) = slot(session, len);
        fill(session, address, &bytes);
        assert_ne!(session.apply_commands(ticket, len), 0);
    }

    #[test]
    fn configured_stack_rejects_neighbours_and_old_epochs_without_publishing() {
        let _clone_guard = clone_guard();
        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        let keys: Vec<u8> = [7_u32, 3_u32]
            .into_iter()
            .flat_map(|frame| {
                let mut key = 5_u64.to_le_bytes().to_vec();
                key.extend_from_slice(&2_u32.to_le_bytes());
                key.extend_from_slice(&frame.to_le_bytes());
                key
            })
            .collect();
        let (ptr, ticket) = slot(&mut session, 32);
        fill(&mut session, ptr, &keys);
        let epoch = session.configure_stack(ticket, 32).unwrap_or(0);
        assert_ne!(epoch, 0);
        assert_eq!(
            session.viewport().scroll().displayed_key(),
            Some(FrameKey::new(SeriesId(5), 2, 7))
        );
        assert_eq!(session.viewport().flags() & 2, 2);
        assert!(matches!(
            session.draw_committed_mono(),
            Err(RasterError::Canvas)
        ));
        let meta = FrameMeta {
            viewport: 7,
            columns: 1,
            rows: 1,
            generation: 0,
        };
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &40_f32.to_le_bytes());
        assert_eq!(
            session.commit_selected_frame(ticket, 4, meta, 0, epoch),
            Ok(())
        );
        assert!(session.draw_committed_mono().is_ok());
        assert_eq!(session.cpu_frame(), &[255, 255, 255, 255]);
        let scroll = command_bytes(11, &1_i32.to_le_bytes());
        let len = u32::try_from(scroll.len()).unwrap_or(0);
        let (ptr, ticket) = slot(&mut session, len);
        fill(&mut session, ptr, &scroll);
        assert_eq!(session.apply_commands(ticket, len), 1);
        assert_eq!(
            session.viewport().scroll().displayed_key(),
            Some(FrameKey::new(SeriesId(5), 2, 3))
        );
        assert_eq!(session.viewport().flags() & 2, 2);
        assert!(matches!(
            session.draw_committed_mono(),
            Err(RasterError::Canvas)
        ));
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &40_f32.to_le_bytes());
        assert_eq!(
            session.commit_selected_frame(ticket, 4, meta, 0, epoch),
            Err(SessionError::Selection)
        );
        assert!(session.last_frame().is_empty());
        assert_eq!(session.bulk_ticket(), 0);
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &0_f32.to_le_bytes());
        assert_eq!(
            session.commit_selected_frame(ticket, 4, meta, 1, epoch),
            Ok(())
        );
        assert!(session.draw_committed_mono().is_ok());
        assert_eq!(session.cpu_frame(), &[0, 0, 0, 255]);
        let (ptr, ticket) = slot(&mut session, 32);
        fill(&mut session, ptr, &keys);
        let next = session.configure_stack(ticket, 32).unwrap_or(0);
        assert_ne!(epoch, next);
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &40_f32.to_le_bytes());
        assert_eq!(
            session.commit_selected_frame(ticket, 4, meta, 0, epoch),
            Err(SessionError::Selection)
        );
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &40_f32.to_le_bytes());
        assert_eq!(
            session.commit_selected_frame(ticket, 4, meta, 0, next),
            Ok(())
        );
        assert_eq!(session.viewport().flags() & 2, 0);
    }

    #[test]
    fn malformed_stack_and_foreign_tickets_preserve_the_installed_sequence() {
        let mut session = Session::new(require_viewport(7));
        let mut key = 1_u64.to_le_bytes().to_vec();
        key.extend_from_slice(&2_u32.to_le_bytes());
        key.extend_from_slice(&3_u32.to_le_bytes());
        let (ptr, ticket) = slot(&mut session, 16);
        fill(&mut session, ptr, &key);
        let epoch = session.configure_stack(ticket, 16).unwrap_or(0);
        let (ptr, ticket) = slot(&mut session, 17);
        fill(&mut session, ptr, &[0_u8; 17]);
        assert_eq!(
            session.configure_stack(ticket.wrapping_add(1), 17),
            Err(SessionError::Stale)
        );
        assert_eq!(session.bulk_ticket(), ticket);
        assert_eq!(
            session.configure_stack(ticket, 17),
            Err(SessionError::Length)
        );
        let (_zero, zero_ticket) = slot(&mut session, 0);
        assert_ne!(zero_ticket, 0);
        assert_eq!(
            session.configure_stack(zero_ticket, 0),
            Err(SessionError::Length)
        );
        assert_eq!(session.stack_epoch, epoch);
        assert_eq!(
            session.viewport().scroll().displayed_key(),
            Some(FrameKey::new(SeriesId(1), 2, 3))
        );
        let (ptr, ticket) = slot(&mut session, 4);
        fill(&mut session, ptr, &40_f32.to_le_bytes());
        let meta = FrameMeta {
            viewport: 7,
            columns: 1,
            rows: 1,
            generation: 0,
        };
        assert_eq!(
            session.commit_frame(ticket, 4, meta),
            Err(SessionError::Selection)
        );
        assert_eq!(session.bulk_ticket(), 0);
    }

    fn command_bytes(kind: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&7_u16.to_le_bytes());
        let len = u32::try_from(payload.len()).unwrap_or(0);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn settle_generation_zero(session: &mut Session) {
        let drawn = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(matches!(drawn, Ok(result) if result.wrote));
    }

    fn assert_next_settle_writes(session: &mut Session) {
        let generation = session.viewport().generation();
        let stale = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(matches!(stale, Err(RasterError::Stale)));
        let drawn = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation },
        );
        assert!(matches!(drawn, Ok(result) if result.wrote && result.extent == [1, 1]));
        let idle = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation },
        );
        assert!(matches!(idle, Ok(result) if !result.wrote && result.extent == [0, 0]));
    }

    #[test]
    fn a_successful_command_or_frame_invalidates_the_settled_cpu_frame() {
        let _clone_guard = clone_guard();
        let cases = [
            command_bytes(9, &[1]),
            command_bytes(7, &{
                let mut payload = Vec::new();
                payload.extend_from_slice(&50.0_f64.to_le_bytes());
                payload.extend_from_slice(&200.0_f64.to_le_bytes());
                payload.push(0);
                payload
            }),
            command_bytes(3, &2.0_f64.to_le_bytes()),
            command_bytes(11, &1_i32.to_le_bytes()),
        ];
        for bytes in cases {
            let mut session = Session::new(require_viewport(7));
            assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
            settle_generation_zero(&mut session);
            let (address, ticket) = slot(&mut session, u32::try_from(bytes.len()).unwrap_or(0));
            fill(&mut session, address, &bytes);
            assert_ne!(
                session.apply_commands(ticket, u32::try_from(bytes.len()).unwrap_or(0)),
                0
            );
            assert_next_settle_writes(&mut session);
        }

        let mut session = Session::new(require_viewport(7));
        assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
        settle_generation_zero(&mut session);
        let (address, ticket) = slot(&mut session, 4);
        fill(&mut session, address, &[1, 2, 3, 4]);
        let meta = FrameMeta {
            viewport: 7,
            columns: 1,
            rows: 1,
            generation: 0,
        };
        assert_eq!(session.commit_frame(ticket, 4, meta), Ok(()));
        assert_next_settle_writes(&mut session);

        let mut refused = Session::new(require_viewport(7));
        assert!(refused.bind_cpu(1.0, 1.0, 1.0).is_ok());
        settle_generation_zero(&mut refused);
        let (address, ticket) = slot(&mut refused, 0);
        assert_ne!(address, 0);
        assert_eq!(refused.apply_commands(ticket, 0), 0);
        let idle = refused.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(matches!(idle, Ok(result) if !result.wrote));
        let (bad_address, bad_ticket) = slot(&mut refused, 4);
        fill(&mut refused, bad_address, &[9, 9, 9, 9]);
        assert_eq!(refused.apply_commands(0, 4), 0);
        assert_eq!(refused.bulk_ticket(), bad_ticket);
        let still = refused.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [1, 1],
            RasterQuality::Settled { generation: 0 },
        );
        assert!(matches!(still, Ok(result) if !result.wrote));
    }

    #[test]
    fn presentation_cell_index_uses_the_full_product() {
        assert_eq!(super::cell_index(1, 4, 2), Ok(0));
        assert_eq!(super::cell_index(3, 4, 2), Ok(1));
        assert_eq!(super::cell_index(65536, 65537, 65537), Ok(65536));
        assert_eq!(super::cell_index(92683, 92684, 46342), Ok(46341));

        let drawn_width = 46_342_u32;
        let device_width = 92_684_u32;
        let drawn_bytes = usize::try_from(drawn_width).unwrap_or(0).saturating_mul(4);
        let device_bytes = usize::try_from(device_width).unwrap_or(0).saturating_mul(4);
        let mut source = vec![0_u8; drawn_bytes];
        let mut destination = vec![0_u8; device_bytes];
        let correct = usize::try_from(46_341_u32).unwrap_or(0).saturating_mul(4);
        let previous = usize::try_from(46_339_u32).unwrap_or(0).saturating_mul(4);
        let Some(correct_px) = source.get_mut(correct..correct + 4) else {
            unreachable!("drawn buffer holds the covering cell");
        };
        correct_px.copy_from_slice(&[1, 2, 3, 4]);
        let Some(previous_px) = source.get_mut(previous..previous + 4) else {
            unreachable!("drawn buffer holds the previous cell");
        };
        previous_px.copy_from_slice(&[9, 9, 9, 9]);
        assert!(
            super::replicate_cells(
                [drawn_width, 1],
                [device_width, 1],
                &source,
                &mut destination,
            )
            .is_ok()
        );
        let at = usize::try_from(92_683_u32).unwrap_or(0).saturating_mul(4);
        assert_eq!(destination.get(at..at + 4), Some(&[1, 2, 3, 4][..]));
    }

    /// F-231's session frame cache. The byte figures are the plan's
    /// hand-computed values: a 512 by 512 frame at one `Stored` per sample is
    /// 512 * 512 * 4 = 1,048,576 bytes, from Rows and Columns in DICOM PS3.3
    /// C.7.6.3.1.
    mod session_cache {
        use ocelli_cache::{
            FrameKey, ImageCache, KIND_BUDGET_CHANGED, KIND_DISPLACED, KIND_EVICTED, KIND_PRESSURE,
            KIND_REFUSED, KIND_RELEASED, SeriesId,
        };
        use ocelli_core::Stored;

        use super::{FrameMeta, Session, SessionError};
        use super::{KIND_PRESENTED, command_bytes, fill, require_viewport, slot};

        /// One frame of the plan's fixtures: 1,048,576 bytes at 512 by 512.
        const FRAME_BYTES: usize = 1_048_576;

        /// Contents that differ above the 32-bit half, so a u32 truncation
        /// anywhere on the path collides them and goes red.
        const CONTENTS: [u64; 4] = [0x1_0000_0001, 0x1_0000_0002, 0x1_0000_0003, 0x1_0000_0004];

        fn key(series: u64, instance: u32) -> FrameKey {
            FrameKey::new(SeriesId(series), instance, 0)
        }

        fn stack_bytes(keys: &[FrameKey]) -> Vec<u8> {
            let mut raw = Vec::new();
            for frame_key in keys {
                raw.extend_from_slice(&frame_key.series.0.to_le_bytes());
                raw.extend_from_slice(&frame_key.instance.to_le_bytes());
                raw.extend_from_slice(&frame_key.frame.to_le_bytes());
            }
            raw
        }

        /// Install `keys` and return the new epoch. Refusal is an `assert`.
        fn install_stack(session: &mut Session, keys: &[FrameKey]) -> u32 {
            let raw = stack_bytes(keys);
            let len = u32::try_from(raw.len()).unwrap_or(0);
            let (address, ticket) = slot(session, len);
            fill(session, address, &raw);
            let epoch = session.configure_stack(ticket, len).unwrap_or(0);
            assert_ne!(epoch, 0);
            epoch
        }

        /// A session with a CPU raster bound and `keys` installed at the
        /// default budget of zero, which is off.
        fn cached_session(keys: &[FrameKey]) -> (Session, u32) {
            let mut session = Session::new(require_viewport(7));
            assert!(session.bind_cpu(1.0, 1.0, 1.0).is_ok());
            let epoch = install_stack(&mut session, keys);
            (session, epoch)
        }

        /// Commit `samples` as one `columns` by `rows` frame at `index` with
        /// `content` naming the bytes. Refusal is an `assert`.
        fn commit_at(
            session: &mut Session,
            index: u32,
            epoch: u32,
            content: u64,
            columns: u32,
            rows: u32,
            samples: &[f32],
        ) {
            let mut bytes = Vec::new();
            for value in samples {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            let len = u32::try_from(bytes.len()).unwrap_or(0);
            let (address, ticket) = slot(session, len);
            fill(session, address, &bytes);
            let meta = FrameMeta {
                viewport: 7,
                columns,
                rows,
                generation: 0,
            };
            assert_eq!(
                session.commit_cached_frame(ticket, len, meta, index, epoch, content),
                Ok(())
            );
        }

        /// Commit one 512 by 512 frame of one repeated sample.
        fn commit_flat(session: &mut Session, index: u32, epoch: u32, content: u64, sample: f32) {
            commit_at(
                session,
                index,
                epoch,
                content,
                512,
                512,
                &vec![sample; 512 * 512],
            );
        }

        /// Scroll by a command, the way the shell does. An index change
        /// clears the displayed source, which is today's behaviour.
        fn scroll(session: &mut Session, step: i32) {
            let bytes = command_bytes(11, &step.to_le_bytes());
            let len = u32::try_from(bytes.len()).unwrap_or(0);
            let (address, ticket) = slot(session, len);
            fill(session, address, &bytes);
            assert_ne!(session.apply_commands(ticket, len), 0);
        }

        /// Drain the ring the way `drain_events` does, header included.
        fn drain(session: &mut Session) -> Vec<u8> {
            let mut out = vec![0_u8; 16 + 256 * 48];
            let copied = session.copy_ring(&mut out);
            out.truncate(copied);
            out
        }

        /// The kind of every event in a drained ring. Events start at 16.
        fn kinds(ring: &[u8]) -> Vec<u32> {
            let mut out = Vec::new();
            let mut at = 16;
            while at + 48 <= ring.len() {
                out.push(super::read_u32(ring, at));
                at += 48;
            }
            out
        }

        fn read_u64(bytes: &[u8], at: usize) -> u64 {
            let mut buf = [0_u8; 8];
            let Some(slot_bytes) = bytes.get(at..at + 8) else {
                return u64::MAX;
            };
            buf.copy_from_slice(slot_bytes);
            u64::from_le_bytes(buf)
        }

        /// Cache a full budget of three frames directly on the field. The
        /// budget change itself is not the observation in these fixtures,
        /// and this keeps their rings free of its event.
        fn with_budget(session: &mut Session, budget: usize) {
            session.frames = ImageCache::new(budget);
        }

        /// Whether the held samples equal `values` bit for bit. The
        /// comparison is on the little-endian bytes, never on floats.
        fn samples_are(samples: &[Stored], values: &[f32]) -> bool {
            samples.len() == values.len()
                && samples
                    .iter()
                    .zip(values.iter())
                    .all(|(stored, value)| stored.0.to_le_bytes() == value.to_le_bytes())
        }

        /// The first held sample's bytes, or zeros when there is none.
        fn first_sample(samples: &[Stored]) -> [u8; 4] {
            match samples.first() {
                Some(stored) => stored.0.to_le_bytes(),
                None => [0; 4],
            }
        }

        /// The plan's frame size as the `u64` the payload fields carry.
        fn frame_bytes(times: usize) -> u64 {
            u64::try_from(FRAME_BYTES)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::try_from(times).unwrap_or(u64::MAX))
        }

        #[test]
        fn four_1mib_commits_report_pressure_then_eviction_in_order() {
            // AC2. Budget 3,145,728 = 3 * 1,048,576. Four commits of 512 by
            // 512 frames: `used` walks 1,048,576, 2,097,152, 3,145,728,
            // 3,145,728. The events are, in order, none, one pressure
            // Elevated, one pressure Critical, then one evicted for index
            // 0's key, and no pressure with the eviction because the band
            // has not moved. Band edges, hand-computed from F-034's integer
            // comparisons: Elevated needs 2 * 2,097,152 = 4,194,304 >=
            // 3,145,728, and 10 * 2,097,152 = 20,971,520 < 28,311,552 = 9 *
            // 3,145,728 keeps it below Critical. The third commit is
            // Critical: 31,457,280 >= 28,311,552.
            let keys: Vec<FrameKey> = (0..4).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 3_145_728);

            commit_flat(&mut session, 0, epoch, CONTENTS[0], 10.0);
            assert_eq!(session.frame_cache_pressure().used, FRAME_BYTES);
            assert_eq!(drain(&mut session).len(), 16);

            scroll(&mut session, 1);
            commit_flat(&mut session, 1, epoch, CONTENTS[1], 20.0);
            assert_eq!(session.frame_cache_pressure().used, 2 * FRAME_BYTES);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            // The pressure payload: the decoded tier at 0, used at 4, budget
            // at 12, entries at 20, evicted and refused at 24 and 28.
            assert_eq!(super::read_u32(&ring, 16 + 16), 1);
            assert_eq!(read_u64(&ring, 16 + 16 + 4), frame_bytes(2));
            assert_eq!(read_u64(&ring, 16 + 16 + 12), 3_145_728);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 20), 2);

            scroll(&mut session, 1);
            commit_flat(&mut session, 2, epoch, CONTENTS[2], 30.0);
            assert_eq!(session.frame_cache_pressure().used, 3_145_728);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(read_u64(&ring, 16 + 16 + 4), 3_145_728);

            scroll(&mut session, 1);
            commit_flat(&mut session, 3, epoch, CONTENTS[3], 40.0);
            assert_eq!(session.frame_cache_pressure().used, 3_145_728);
            let ring = drain(&mut session);
            // The evicted payload: index 0's key, series 5 at 0, instance at
            // 8, frame at 12 and 1,048,576 bytes at 16.
            assert_eq!(kinds(&ring), [KIND_EVICTED]);
            assert_eq!(read_u64(&ring, 16 + 16), 5);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 0);
            assert_eq!(read_u64(&ring, 16 + 16 + 16), frame_bytes(1));
        }

        #[test]
        fn a_default_budget_caches_nothing_and_leaves_the_ring_to_kind_6() {
            // AC3. No set_frame_budget: the default budget is 0, which is
            // off. A cached commit displays as today, holds nothing and
            // emits nothing, so the next event the ring carries is a
            // presented frame and nothing else.
            let (mut session, epoch) = cached_session(&[key(5, 0), key(5, 1)]);
            commit_flat(&mut session, 0, epoch, CONTENTS[0], 10.0);
            assert_eq!(session.frame_cache_pressure().budget, 0);
            assert_eq!(session.frame_cache_pressure().used, 0);
            assert_eq!(drain(&mut session).len(), 16);
            assert_ne!(session.present(1, 1), u64::MAX);
            assert_eq!(kinds(&drain(&mut session)), [KIND_PRESENTED]);
        }

        #[test]
        fn residency_is_not_a_use() {
            // AC4. 1 for the matching triple, 0 for another content, other
            // columns or other rows, and 0 under a stale epoch. The query
            // moves neither recency nor used, so the next eviction is still
            // index 0's key: a query that marked a use would lift that entry
            // above its neighbours and evict index 1's key instead.
            let keys: Vec<FrameKey> = (0..4).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 3_145_728);
            for (index, (content, sample)) in
                CONTENTS.iter().zip([10.0_f32, 20.0, 30.0]).enumerate()
            {
                if index > 0 {
                    scroll(&mut session, 1);
                }
                commit_flat(
                    &mut session,
                    u32::try_from(index).unwrap_or(0),
                    epoch,
                    *content,
                    sample,
                );
            }
            assert_eq!(session.frame_cache_pressure().used, 3_145_728);
            // The three inserts' own events are setup, not the observation.
            drain(&mut session);

            let request = (0_u32, epoch, CONTENTS[0], 512_u32, 512_u32);
            assert!(session.frame_resident(request.0, request.1, request.2, request.3, request.4));
            // Another content, including one that differs only above the
            // 32-bit half.
            assert!(!session.frame_resident(0, epoch, CONTENTS[0] ^ 1, 512, 512));
            assert!(!session.frame_resident(0, epoch, 0x2_0000_0001, 512, 512));
            assert!(!session.frame_resident(0, epoch, CONTENTS[0], 511, 512));
            assert!(!session.frame_resident(0, epoch, CONTENTS[0], 512, 513));
            // The mismatched queries kept the entry.
            assert!(session.frame_resident(0, epoch, CONTENTS[0], 512, 512));
            // A stale epoch and epoch 0 answer 0.
            assert!(!session.frame_resident(0, epoch.wrapping_add(1), CONTENTS[0], 512, 512));
            assert!(!session.frame_resident(0, 0, CONTENTS[0], 512, 512));
            // Nothing about the queries changed the bytes held.
            assert_eq!(session.frame_cache_pressure().used, 3_145_728);

            // The next eviction is still index 0's key.
            scroll(&mut session, 1);
            commit_flat(&mut session, 3, epoch, CONTENTS[3], 40.0);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_EVICTED]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 0);
        }

        #[test]
        fn show_resident_displays_and_stale_refuses() {
            // AC5 and D1. One-sample frames of 4 bytes against a 12-byte
            // budget. Index 2 is displayed and selected. A show of index 1
            // while index 2 is selected is the Selection refusal
            // commit_selected_frame gives: nothing is drawn, the loading
            // flag is as the commit left it and the entry stays. Scrolled
            // to index 1, a mismatched show removes the entry, returns 0
            // and draws nothing, and the removal drops used from 12 to 8,
            // which crosses Critical to Elevated: 10 * 8 = 80 < 108 = 9 *
            // 12, so one pressure event follows. A matching show puts
            // index 1's samples in stored_samples and their little-endian
            // bytes in last_frame, clears loading and draws.
            let keys: Vec<FrameKey> = (0..4).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 12);
            commit_at(&mut session, 0, epoch, CONTENTS[0], 1, 1, &[10.0]);
            scroll(&mut session, 1);
            commit_at(&mut session, 1, epoch, CONTENTS[1], 1, 1, &[20.0]);
            scroll(&mut session, 1);
            commit_at(&mut session, 2, epoch, CONTENTS[2], 1, 1, &[30.0]);
            assert_eq!(session.frame_cache_pressure().used, 12);
            drain(&mut session);

            // The unselected show. The display is index 2's and stays so,
            // and the entry is kept for the show that follows.
            assert_eq!(
                session.show_resident(1, epoch, CONTENTS[1] ^ 1, 1, 1),
                Err(SessionError::Selection)
            );
            assert!(samples_are(session.stored_samples(), &[30.0]));
            assert_eq!(session.frame_cache_pressure().used, 12);
            assert!(session.frame_resident(1, epoch, CONTENTS[1], 1, 1));
            assert_eq!(drain(&mut session).len(), 16);

            // The selection is index 1 now. The stale show removes the
            // entry and draws nothing.
            scroll(&mut session, -1);
            assert_eq!(
                session.show_resident(1, epoch, CONTENTS[1] ^ 1, 1, 1),
                Ok(0)
            );
            assert!(session.stored_samples().is_empty());
            assert_eq!(session.frame_cache_pressure().used, 8);
            assert_eq!(kinds(&drain(&mut session)), [KIND_PRESSURE]);
            assert!(!session.frame_resident(1, epoch, CONTENTS[1], 1, 1));

            // Put the entry back, so the success path below is the thing
            // that answers both. The selection is already index 1.
            commit_at(&mut session, 1, epoch, CONTENTS[1], 1, 1, &[20.0]);

            assert_eq!(session.show_resident(1, epoch, CONTENTS[1], 1, 1), Ok(1));
            assert!(samples_are(session.stored_samples(), &[20.0]));
            assert_eq!(session.last_frame(), &20.0_f32.to_le_bytes());
            // FLAG_LOADING is bit 2, and the show cleared it.
            assert_eq!(session.viewport().flags() & 2, 0);
            let drawn = session.draw_committed_mono();
            assert!(matches!(drawn, Ok(result) if result.wrote && result.extent == [1, 1]));
            assert_eq!(session.frame_cache_pressure().used, 12);
        }

        #[test]
        fn show_resident_refuses_an_index_that_is_not_the_selection() {
            // D1's reproduction from the review. Slice 1 is cached and the
            // selection is slice 0 after the scroll back. A show of slice 1
            // is the Selection refusal, draws nothing and leaves the
            // loading flag as the scroll left it. The code before the fix
            // returned 1 here, adopted slice 1's samples and cleared the
            // loading flag while copy_state still reported slice 0.
            let (mut session, epoch) = cached_session(&[key(5, 0), key(5, 1)]);
            with_budget(&mut session, 8);
            commit_at(&mut session, 0, epoch, CONTENTS[0], 1, 1, &[10.0]);
            scroll(&mut session, 1);
            commit_at(&mut session, 1, epoch, CONTENTS[1], 1, 1, &[20.0]);
            scroll(&mut session, -1);
            drain(&mut session);
            assert_eq!(session.viewport().scroll().index(), Some(0));
            assert_eq!(
                session.show_resident(1, epoch, CONTENTS[1], 1, 1),
                Err(SessionError::Selection)
            );
            assert!(session.stored_samples().is_empty());
            // FLAG_LOADING is bit 2, and the scroll left it set.
            assert_eq!(session.viewport().flags() & 2, 2);
            assert!(session.frame_resident(1, epoch, CONTENTS[1], 1, 1));
            assert_eq!(drain(&mut session).len(), 16);
        }

        #[test]
        fn a_stack_change_keeps_the_cache_and_the_epoch_gates_it() {
            // AC6. configure_stack with the same keys keeps the cache: the
            // display is cleared and residency under the new epoch is 1,
            // under the old epoch 0, and the kept frame still shows.
            let keys = [key(5, 0), key(5, 1)];
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 8);
            commit_at(&mut session, 0, epoch, CONTENTS[0], 1, 1, &[10.0]);
            assert!(session.frame_resident(0, epoch, CONTENTS[0], 1, 1));

            let next = install_stack(&mut session, &keys);
            assert_ne!(epoch, next);
            assert!(session.stored_samples().is_empty());
            assert!(session.frame_resident(0, next, CONTENTS[0], 1, 1));
            assert!(!session.frame_resident(0, epoch, CONTENTS[0], 1, 1));
            assert_eq!(session.show_resident(0, next, CONTENTS[0], 1, 1), Ok(1));
            assert!(samples_are(session.stored_samples(), &[10.0]));
        }

        #[test]
        fn release_series_reports_one_event_and_keeps_the_display() {
            // AC7. Budget 4,194,304 holds three 1,048,576-byte frames of
            // series 5 and one of series 6, so used reads 4,194,304 and the
            // band is Critical: 41,943,040 >= 37,748,736. release_series(5)
            // returns 3, used drops to 1,048,576, and the ring holds one
            // released event with frames 3 and bytes 3,145,728, then one
            // pressure Low, because 2,097,152 < 4,194,304. The displayed
            // frame is outside the cache and stays.
            let keys = [key(5, 0), key(5, 1), key(5, 2), key(6, 3)];
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 4_194_304);
            for (index, (content, sample)) in CONTENTS
                .iter()
                .zip([10.0_f32, 20.0, 30.0, 40.0])
                .enumerate()
            {
                if index > 0 {
                    scroll(&mut session, 1);
                }
                commit_flat(
                    &mut session,
                    u32::try_from(index).unwrap_or(0),
                    epoch,
                    *content,
                    sample,
                );
            }
            assert_eq!(session.frame_cache_pressure().used, 4_194_304);
            drain(&mut session);
            assert_eq!(
                first_sample(session.stored_samples()),
                40.0_f32.to_le_bytes()
            );

            assert_eq!(session.release_series(5), 3);
            assert_eq!(session.frame_cache_pressure().used, FRAME_BYTES);
            assert_eq!(
                first_sample(session.stored_samples()),
                40.0_f32.to_le_bytes()
            );
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_RELEASED, KIND_PRESSURE]);
            // The released payload: series at 0, frames at 8, bytes at 16.
            assert_eq!(read_u64(&ring, 16 + 16), 5);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 3);
            assert_eq!(read_u64(&ring, 16 + 16 + 16), frame_bytes(3));
            // The pressure reading is Low: 1,048,576 of 4,194,304.
            let second = 16 + 48;
            assert_eq!(super::read_u32(&ring, second + 16), 1);
            assert_eq!(read_u64(&ring, second + 16 + 4), frame_bytes(1));
            assert_eq!(read_u64(&ring, second + 16 + 12), 4_194_304);
        }

        #[test]
        fn a_shrunk_budget_evicts_then_reports_the_budget_change() {
            // AC8. set_frame_budget(1,048,576) with three frames cached
            // evicts the two least recently used, then reports one
            // budgetChanged with budget 1,048,576, evicted 2 and evicted
            // bytes 2,097,152. The band was Critical at 3,145,728 used and
            // is Critical again at 1,048,576 of 1,048,576, because 10 *
            // 1,048,576 >= 9 * 1,048,576 either way, so no pressure event
            // follows. BudgetChanged before the evictions would be against
            // F-034's order and would go red here.
            let keys: Vec<FrameKey> = (0..3).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 3_145_728);
            for (index, (content, sample)) in
                CONTENTS.iter().zip([10.0_f32, 20.0, 30.0]).enumerate()
            {
                if index > 0 {
                    scroll(&mut session, 1);
                }
                commit_flat(
                    &mut session,
                    u32::try_from(index).unwrap_or(0),
                    epoch,
                    *content,
                    sample,
                );
            }
            drain(&mut session);

            assert_eq!(
                session.set_frame_budget(u32::try_from(FRAME_BYTES).unwrap_or(0)),
                1
            );
            let ring = drain(&mut session);
            assert_eq!(
                kinds(&ring),
                [KIND_EVICTED, KIND_EVICTED, KIND_BUDGET_CHANGED]
            );
            // The evictions are least recently used first: index 0's key,
            // then index 1's, each at 1,048,576 bytes.
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 0);
            assert_eq!(read_u64(&ring, 16 + 16 + 16), frame_bytes(1));
            let second = 16 + 48;
            assert_eq!(super::read_u32(&ring, second + 16 + 8), 1);
            assert_eq!(read_u64(&ring, second + 16 + 16), frame_bytes(1));
            // The budget change: the decoded tier at 0, budget at 4, evicted
            // bytes at 12, evicted entries at 20 and dropped at 24.
            let third = 16 + 2 * 48;
            assert_eq!(super::read_u32(&ring, third + 16), 1);
            assert_eq!(read_u64(&ring, third + 16 + 4), frame_bytes(1));
            assert_eq!(read_u64(&ring, third + 16 + 12), frame_bytes(2));
            assert_eq!(super::read_u32(&ring, third + 16 + 20), 2);
            assert_eq!(session.frame_cache_pressure().used, FRAME_BYTES);
            // And no pressure event: the band did not cross.
            assert_eq!(drain(&mut session).len(), 16);
        }

        #[test]
        fn a_budget_change_that_crosses_a_band_reports_the_pressure_last() {
            // S1. Two 1,048,576-byte frames of a 2,097,152-byte budget are
            // Critical: 10 * 2,097,152 >= 9 * 2,097,152. set_frame_budget(0)
            // evicts both, least recently used first, then one
            // BudgetChanged, then the pressure the crossing to Low pushes,
            // because 0 of 0 is Low. Removing the trailing band call after
            // the budget change leaves this list at three events and goes
            // red here.
            let keys: Vec<FrameKey> = (0..2).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 2_097_152);
            commit_flat(&mut session, 0, epoch, CONTENTS[0], 10.0);
            scroll(&mut session, 1);
            commit_flat(&mut session, 1, epoch, CONTENTS[1], 20.0);
            drain(&mut session);

            assert_eq!(session.set_frame_budget(0), 1);
            let ring = drain(&mut session);
            assert_eq!(
                kinds(&ring),
                [
                    KIND_EVICTED,
                    KIND_EVICTED,
                    KIND_BUDGET_CHANGED,
                    KIND_PRESSURE
                ]
            );
            // The evictions are least recently used first: index 0's key,
            // then index 1's, each at 1,048,576 bytes.
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 0);
            assert_eq!(read_u64(&ring, 16 + 16 + 16), frame_bytes(1));
            let second = 16 + 48;
            assert_eq!(super::read_u32(&ring, second + 16 + 8), 1);
            assert_eq!(read_u64(&ring, second + 16 + 16), frame_bytes(1));
            // The budget change: the decoded tier at 0, budget 0 at 4,
            // evicted bytes at 12 and evicted entries at 20.
            let third = 16 + 2 * 48;
            assert_eq!(super::read_u32(&ring, third + 16), 1);
            assert_eq!(read_u64(&ring, third + 16 + 4), 0);
            assert_eq!(read_u64(&ring, third + 16 + 12), frame_bytes(2));
            assert_eq!(super::read_u32(&ring, third + 16 + 20), 2);
            // The pressure reading the crossing pushed: used 0 of 0, and
            // the two evictions this change returned are its counts.
            let fourth = 16 + 3 * 48;
            assert_eq!(super::read_u32(&ring, fourth + 16), 1);
            assert_eq!(read_u64(&ring, fourth + 16 + 4), 0);
            assert_eq!(read_u64(&ring, fourth + 16 + 12), 0);
            assert_eq!(super::read_u32(&ring, fourth + 16 + 24), 2);
            assert_eq!(session.frame_cache_pressure().used, 0);
        }

        #[test]
        fn a_non_multiple_of_four_and_a_contentless_commit_are_not_cached() {
            // AC9. A commit whose length is not a multiple of 4 has no
            // stored samples and is displayed as today, not cached. A commit
            // through commit_selected_frame carries no content id and is not
            // cached either. Neither emits an event.
            let (mut session, epoch) = cached_session(&[key(5, 0)]);
            with_budget(&mut session, 64);
            let meta = FrameMeta {
                viewport: 7,
                columns: 1,
                rows: 1,
                generation: 0,
            };
            let (address, ticket) = slot(&mut session, 3);
            fill(&mut session, address, &[1, 2, 3]);
            assert_eq!(
                session.commit_cached_frame(ticket, 3, meta, 0, epoch, CONTENTS[0]),
                Ok(())
            );
            assert!(session.stored_samples().is_empty());
            assert_eq!(session.frame_cache_pressure().used, 0);
            assert_eq!(drain(&mut session).len(), 16);

            let (address, ticket) = slot(&mut session, 4);
            fill(&mut session, address, &40_f32.to_le_bytes());
            assert_eq!(
                session.commit_selected_frame(ticket, 4, meta, 0, epoch),
                Ok(())
            );
            assert!(samples_are(session.stored_samples(), &[40.0]));
            assert_eq!(session.frame_cache_pressure().used, 0);
            assert_eq!(drain(&mut session).len(), 16);
        }

        #[test]
        fn a_frame_larger_than_the_whole_budget_is_refused_and_displayed() {
            // AC10. A budget of 4 bytes holds one one-sample frame. A
            // two-sample frame is 8 bytes: refused with one cache.refused
            // naming index 1's key and 8 bytes, never admitted against D-24,
            // and displayed as today with used unchanged at 4.
            let (mut session, epoch) = cached_session(&[key(5, 0), key(5, 1)]);
            with_budget(&mut session, 4);
            commit_at(&mut session, 0, epoch, CONTENTS[0], 1, 1, &[10.0]);
            assert_eq!(session.frame_cache_pressure().used, 4);
            drain(&mut session);

            scroll(&mut session, 1);
            commit_at(&mut session, 1, epoch, CONTENTS[1], 2, 1, &[10.0, 20.0]);
            assert!(samples_are(session.stored_samples(), &[10.0, 20.0]));
            assert_eq!(session.frame_cache_pressure().used, 4);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_REFUSED]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 8), 1);
            assert_eq!(read_u64(&ring, 16 + 16 + 16), 8);

            // D3. The refusal is counted for the next reading: refused 1 at
            // payload offset 28, and no eviction at 24, because the insert
            // was refused before anything left. Before the fix the reading
            // reported constant zeros.
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 1);
        }

        #[test]
        fn a_pressure_reading_carries_the_counts_since_the_previous_one() {
            // D3. AC2's walk: budget 3,145,728 and four 512 by 512 commits.
            // The second and the third commit push the Elevated and the
            // Critical crossing readings, and the fourth evicts index 0's
            // entry with no crossing of its own. A requested reading then
            // reports evicted 1 at payload offset 24, and a second reading
            // reports 0: the counts are F-034's TierCounters, taken at each
            // pressure push. Before the fix both readings reported
            // constant zeros.
            let keys: Vec<FrameKey> = (0..4).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 3_145_728);
            for (index, (content, sample)) in CONTENTS
                .iter()
                .zip([10.0_f32, 20.0, 30.0, 40.0])
                .enumerate()
            {
                if index > 0 {
                    scroll(&mut session, 1);
                }
                commit_flat(
                    &mut session,
                    u32::try_from(index).unwrap_or(0),
                    epoch,
                    *content,
                    sample,
                );
            }
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE, KIND_PRESSURE, KIND_EVICTED]);

            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 1);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);

            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);
        }

        #[test]
        fn displaced_and_released_frames_are_not_counted_as_evictions() {
            // S1. The review's zz_probe_d3 walk on 4-byte frames against an
            // 8-byte budget: the evicting commit's reading reports evicted 1
            // at payload offset 24, a repeat commit of the same content
            // displaces and the next reading reports 0, a stale removal
            // pushes a crossing reading with 0, and the release pushes one
            // with 0 too. Counting a Displaced or a Released event as an
            // eviction in note_counters goes red here.
            let keys: Vec<FrameKey> = (0..3).map(|instance| key(5, instance)).collect();
            let (mut session, epoch) = cached_session(&keys);
            with_budget(&mut session, 8);
            for (index, (content, sample)) in
                CONTENTS.iter().zip([10.0_f32, 20.0, 30.0]).enumerate()
            {
                if index > 0 {
                    scroll(&mut session, 1);
                }
                commit_at(
                    &mut session,
                    u32::try_from(index).unwrap_or(0),
                    epoch,
                    *content,
                    1,
                    1,
                    &[sample],
                );
            }
            assert_eq!(session.frame_cache_pressure().used, 8);
            // The walk itself: two band crossings and index 0's eviction.
            assert_eq!(
                kinds(&drain(&mut session)),
                [KIND_PRESSURE, KIND_PRESSURE, KIND_EVICTED]
            );

            // The evicting commit's reading reports it at payload offset 24.
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 1);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);

            // The reading resets, so the next one starts from zero.
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);

            // A repeat commit of the same content at the resident index 1
            // displaces and is not an eviction. The selection moves back one
            // first, because a cached commit refuses another index.
            scroll(&mut session, -1);
            commit_at(&mut session, 1, epoch, CONTENTS[1], 1, 1, &[20.0]);
            assert_eq!(kinds(&drain(&mut session)), [KIND_DISPLACED]);
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);

            // A stale removal at the selected index emits no event at all,
            // and the crossing reading it pushes counts nothing.
            assert_eq!(
                session.show_resident(1, epoch, CONTENTS[1] ^ 1, 1, 1),
                Ok(0)
            );
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 28), 0);
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(super::read_u32(&ring, 16 + 16 + 24), 0);

            // The release pushes its released event and a crossing reading,
            // and neither is an eviction. The reading is the second event,
            // so its counts are read at 16 + 48 into the ring.
            assert_eq!(session.release_series(5), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_RELEASED, KIND_PRESSURE]);
            let second = 16 + 48;
            assert_eq!(super::read_u32(&ring, second + 16 + 24), 0);
            assert_eq!(super::read_u32(&ring, second + 16 + 28), 0);
        }

        #[test]
        fn a_burst_over_the_ring_capacity_counts_the_dropped() {
            // AC11's session half. 300 commits of one index displace 300
            // events into an empty ring of 256: 256 are pushed and 44 are
            // counted in dropped, which drain_events reports in its header.
            let (mut session, epoch) = cached_session(&[key(5, 0)]);
            with_budget(&mut session, 4);
            for _ in 0..300 {
                commit_at(&mut session, 0, epoch, CONTENTS[0], 1, 1, &[10.0]);
            }
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring).len(), 256);
            assert_eq!(super::read_u32(&ring, 0), 256);
            assert_eq!(super::read_u32(&ring, 12), 44);
        }

        #[test]
        fn content_id_hashes_the_slot_and_drops_it() {
            // Decision 8. The slot holds the SOP Instance UID, the transfer
            // syntax UID and the encoded bytes back to back, and the two
            // lengths name the first two fields. The value over these three
            // fields is F-224 AC2's, recomputed by hand from the FNV-1a
            // definition with the length prefixes. A refusal answers None,
            // because 0 is a legitimate id, and a foreign ticket leaves the
            // slot in place.
            let (mut session, _epoch) = cached_session(&[key(5, 0)]);
            assert_eq!(session.content_id(0, 0, 0, 0), None);

            let uid = b"1.2.3";
            let transfer_syntax = b"1.2.840.10008.1.2.1";
            let mut raw = Vec::new();
            raw.extend_from_slice(uid);
            raw.extend_from_slice(transfer_syntax);
            raw.extend_from_slice(&[0, 1, 2, 3]);
            let len = u32::try_from(raw.len()).unwrap_or(0);
            let (address, ticket) = slot(&mut session, len);
            fill(&mut session, address, &raw);
            assert_eq!(session.bulk_ticket(), ticket);
            assert_eq!(session.content_id(ticket.wrapping_add(1), len, 5, 19), None);
            assert_eq!(session.bulk_ticket(), ticket);

            assert_eq!(
                session.content_id(ticket, len, 5, 19),
                Some(0x49e8_b650_ff1c_d3f1)
            );
            // The slot is dropped, so the ticket is no longer current.
            assert_eq!(session.bulk_ticket(), 0);

            // Lengths that overrun the slot are refused and drop it.
            let (address, ticket) = slot(&mut session, len);
            fill(&mut session, address, &raw);
            assert_eq!(session.content_id(ticket, len, len, 1), None);
            assert_eq!(session.bulk_ticket(), 0);
        }

        #[test]
        fn request_frame_pressure_pushes_one_reading() {
            // Decision 10. A requested reading is a pressure event with the
            // current reading and nothing else. With the budget off the
            // reading is 0 of 0.
            let (mut session, _epoch) = cached_session(&[key(5, 0)]);
            assert_eq!(session.request_frame_pressure(), 1);
            let ring = drain(&mut session);
            assert_eq!(kinds(&ring), [KIND_PRESSURE]);
            assert_eq!(read_u64(&ring, 16 + 16 + 4), 0);
            assert_eq!(read_u64(&ring, 16 + 16 + 12), 0);
        }

        #[test]
        fn a_repeat_commit_of_the_same_content_displaces() {
            // Decision 4's repeat case. The same index and the same content
            // displace the held frame with equal samples and emit one
            // displaced event, and used stays at one frame.
            let (mut session, epoch) = cached_session(&[key(5, 0)]);
            with_budget(&mut session, FRAME_BYTES);
            commit_flat(&mut session, 0, epoch, CONTENTS[0], 10.0);
            drain(&mut session);
            commit_flat(&mut session, 0, epoch, CONTENTS[0], 10.0);
            assert_eq!(session.frame_cache_pressure().used, FRAME_BYTES);
            assert_eq!(kinds(&drain(&mut session)).len(), 1);
        }
    }
}
