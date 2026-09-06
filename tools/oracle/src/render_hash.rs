//! Stable identity for the exact RGBA8 output the comparator reads.
//!
//! HLD section 38 requires stable render hashes as a Phase 1 hook. Decision
//! D14 separately requires an attestation to claim measured divergence rather
//! than bit-exact reproducibility. These hashes therefore say one narrow thing:
//! equal hashes identify equal, shape-aware output bytes under this versioned
//! contract. They do not define a tolerance and they do not excuse inequality.
//!
//! Every field is framed as an unsigned 64-bit little-endian byte length
//! followed by the bytes. Dimensions are unsigned 32-bit little-endian values
//! inside that framing. The run hash sorts by kind and identifier, then frames
//! each view's algorithm digest. Report ordering and elapsed time never enter.

use sha2::{Digest, Sha256};

use crate::frame::Frame;
use crate::sidecar::ViewKind;

/// The public version token written beside every hash.
pub const ALGORITHM: &str = "sha256-rgba8-v1";

const VIEW_DOMAIN: &[u8] = b"sha256-rgba8-v1\0";
const RUN_DOMAIN: &[u8] = b"sha256-rgba8-run-v1\0";
const FORMAT: &[u8] = b"RGBA8";

/// One view's stable render identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderHash {
    pub kind: ViewKind,
    pub id: String,
    pub sha256: String,
}

fn frame_field(hasher: &mut Sha256, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    hasher.update(length.to_le_bytes());
    hasher.update(bytes);
}

fn finish(hasher: Sha256) -> String {
    format!("{:x}", hasher.finalize())
}

/// Hash one already validated, tightly packed RGBA8 frame.
#[must_use]
pub fn render_hash(kind: ViewKind, id: &str, frame: &Frame) -> RenderHash {
    let mut hasher = Sha256::new();
    hasher.update(VIEW_DOMAIN);
    frame_field(&mut hasher, kind.label().as_bytes());
    frame_field(&mut hasher, id.as_bytes());
    frame_field(&mut hasher, &frame.width().to_le_bytes());
    frame_field(&mut hasher, &frame.height().to_le_bytes());
    frame_field(&mut hasher, FORMAT);
    frame_field(&mut hasher, frame.bytes());
    RenderHash {
        kind,
        id: id.to_owned(),
        sha256: finish(hasher),
    }
}

/// Hash a canonical ordered list of per-view render identities.
#[must_use]
pub fn run_render_hash(hashes: &[RenderHash]) -> String {
    let mut ordered: Vec<&RenderHash> = hashes.iter().collect();
    ordered.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.sha256.cmp(&right.sha256))
    });

    let mut hasher = Sha256::new();
    hasher.update(RUN_DOMAIN);
    hasher.update(
        u64::try_from(ordered.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for hash in ordered {
        frame_field(&mut hasher, hash.kind.label().as_bytes());
        frame_field(&mut hasher, hash.id.as_bytes());
        frame_field(&mut hasher, hash.sha256.as_bytes());
    }
    finish(hasher)
}
