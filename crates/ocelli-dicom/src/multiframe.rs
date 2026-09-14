//! Enhanced multiframe projection over lossless DICOM metadata.
//!
//! DICOM PS3.3 C.7.6.6 and C.7.6.16 define Number of Frames and functional
//! group lookup. This module retains source evidence without interpreting the
//! geometry, stack, temporal, or calibration attributes that F-020 owns.

use dicom_core::{Tag, VR};

use crate::{MetadataElement, MetadataSet, MetadataValue};

const NUMBER_OF_FRAMES: Tag = Tag(0x0028, 0x0008);
const SHARED_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9229);
const PER_FRAME_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9230);

/// A structural multiframe metadata failure.
///
/// Variants retain only tags, counts, and source labels. Attribute values are
/// never copied into an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MultiframeError {
    /// Number of Frames is present with a VR other than IS.
    #[error("Number of Frames does not use IS")]
    InvalidNumberOfFramesVr(VR),
    /// Number of Frames is present but empty.
    #[error("Number of Frames is empty")]
    EmptyNumberOfFrames,
    /// Number of Frames contains more than one value.
    #[error("Number of Frames contains {0} values")]
    MultipleNumberOfFrames(usize),
    /// Number of Frames is not a positive base-10 integer.
    #[error("Number of Frames is not a positive integer")]
    InvalidNumberOfFrames,
    /// Number of Frames declares zero frames.
    #[error("Number of Frames declares zero frames")]
    ZeroNumberOfFrames,
    /// Number of Frames does not fit this target's index width.
    #[error("Number of Frames exceeds the target index width")]
    NumberOfFramesOverflow,
    /// Shared Functional Groups is not an SQ value.
    #[error("Shared Functional Groups is not a sequence")]
    InvalidSharedFunctionalGroups,
    /// Shared Functional Groups contains more than its permitted single item.
    #[error("Shared Functional Groups contains {0} items")]
    SharedFunctionalGroupsCount(usize),
    /// Per-frame Functional Groups is not an SQ value.
    #[error("Per-frame Functional Groups is not a sequence")]
    InvalidPerFrameFunctionalGroups,
    /// Per-frame item count differs from Number of Frames.
    #[error("Per-frame Functional Groups count does not match Number of Frames")]
    PerFrameFunctionalGroupsCount {
        /// The positive declared frame count.
        declared: usize,
        /// The sequence's actual item count.
        actual: usize,
    },
    /// A requested zero-based frame is not present.
    #[error("frame index is outside the declared frame count")]
    FrameOutOfRange {
        /// The requested zero-based frame.
        requested: usize,
        /// The positive declared frame count.
        declared: usize,
    },
    /// A present functional group is not a one-item SQ.
    #[error("functional group is not a one-item sequence")]
    InvalidFunctionalGroup {
        /// The source containing the malformed group.
        location: FunctionalGroupSource,
        /// The functional group sequence tag.
        tag: Tag,
    },
}

/// The source of one resolved functional-group attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionalGroupSource {
    /// The main data set outside functional-group sequences.
    TopLevel,
    /// The Shared Functional Groups item.
    Shared,
    /// The item for one zero-based frame.
    PerFrame(usize),
}

/// Whether the attribute's module permits legacy top-level fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopLevelFallback {
    /// Only functional-group sources are permitted.
    Disallowed,
    /// Top-level metadata may answer after per-frame and shared sources.
    Allowed,
}

/// Duplicate sources found behind the effective value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DuplicateSources(u8);

impl DuplicateSources {
    const TOP_LEVEL: u8 = 1;
    const SHARED: u8 = 2;
    const PER_FRAME: u8 = 4;

    /// Whether this set contains the source kind.
    ///
    /// Any [`FunctionalGroupSource::PerFrame`] value queries the per-frame
    /// source kind. Resolution is already scoped to one requested frame.
    #[must_use]
    pub const fn contains(self, source: FunctionalGroupSource) -> bool {
        let bit = match source {
            FunctionalGroupSource::TopLevel => Self::TOP_LEVEL,
            FunctionalGroupSource::Shared => Self::SHARED,
            FunctionalGroupSource::PerFrame(_) => Self::PER_FRAME,
        };
        self.0 & bit != 0
    }

    /// Whether no lower-precedence duplicate was found.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Every source kind present in either set.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn with(mut self, source: FunctionalGroupSource) -> Self {
        self.0 |= match source {
            FunctionalGroupSource::TopLevel => Self::TOP_LEVEL,
            FunctionalGroupSource::Shared => Self::SHARED,
            FunctionalGroupSource::PerFrame(_) => Self::PER_FRAME,
        };
        self
    }
}

/// One effective attribute plus its retained source evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedFunctionalGroup<'a> {
    element: &'a MetadataElement,
    source: FunctionalGroupSource,
    duplicate_sources: DuplicateSources,
}

impl<'a> ResolvedFunctionalGroup<'a> {
    /// The borrowed lossless metadata element.
    #[must_use]
    pub const fn element(self) -> &'a MetadataElement {
        self.element
    }

    /// The winning source after per-frame, shared, then permitted top-level.
    #[must_use]
    pub const fn source(self) -> FunctionalGroupSource {
        self.source
    }

    /// Lower-precedence sources that also declared the attribute.
    #[must_use]
    pub const fn duplicate_sources(self) -> DuplicateSources {
        self.duplicate_sources
    }
}

/// A checked, borrowed view of one instance's multiframe metadata.
#[derive(Debug, Clone, Copy)]
pub struct MultiframeMetadata<'a> {
    metadata: &'a MetadataSet,
    frame_count: usize,
    shared: Option<&'a MetadataSet>,
    per_frame: Option<&'a [MetadataSet]>,
}

impl<'a> MultiframeMetadata<'a> {
    /// Validate the frame declaration and functional-group item counts.
    ///
    /// # Errors
    ///
    /// Returns a structural error for a present malformed Number of Frames or
    /// functional-group sequence. Number of Frames defaults to one only when
    /// absent.
    pub fn new(metadata: &'a MetadataSet) -> Result<Self, MultiframeError> {
        let frame_count = parse_frame_count(metadata)?;
        let shared = shared_item(metadata)?;
        let per_frame = per_frame_items(metadata)?;
        if let Some(items) = per_frame
            && items.len() != frame_count
        {
            return Err(MultiframeError::PerFrameFunctionalGroupsCount {
                declared: frame_count,
                actual: items.len(),
            });
        }
        Ok(Self {
            metadata,
            frame_count,
            shared,
            per_frame,
        })
    }

    /// The positive declared frame count, or one when the tag was absent.
    #[must_use]
    pub const fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// The borrowed main data set.
    ///
    /// A consumer needs it for attributes that belong to no functional group
    /// macro, such as Rows, Columns and Gantry/Detector Tilt.
    #[must_use]
    pub const fn metadata(&self) -> &'a MetadataSet {
        self.metadata
    }

    /// Resolve one attribute without interpreting its value.
    ///
    /// DICOM PS3.3 C.7.6.16 gives per-frame values precedence over shared
    /// values. A caller must explicitly permit a top-level fallback because
    /// the attribute's owning module decides whether that source is legal.
    /// Duplicate declarations remain observable on the returned value.
    ///
    /// # Errors
    ///
    /// Returns [`MultiframeError::FrameOutOfRange`] for an invalid frame or
    /// [`MultiframeError::InvalidFunctionalGroup`] when a present group is not
    /// a one-item sequence.
    pub fn resolve(
        &self,
        frame: usize,
        group_tag: Tag,
        attribute_tag: Tag,
        top_level: TopLevelFallback,
    ) -> Result<Option<ResolvedFunctionalGroup<'a>>, MultiframeError> {
        if frame >= self.frame_count {
            return Err(MultiframeError::FrameOutOfRange {
                requested: frame,
                declared: self.frame_count,
            });
        }

        let per_frame_source = FunctionalGroupSource::PerFrame(frame);
        let per_frame = match self.per_frame.and_then(|items| items.get(frame)) {
            Some(item) => attribute_in_group(item, group_tag, attribute_tag, per_frame_source)?,
            None => None,
        };
        let shared = match self.shared {
            Some(item) => attribute_in_group(
                item,
                group_tag,
                attribute_tag,
                FunctionalGroupSource::Shared,
            )?,
            None => None,
        };
        let top_level_element = match top_level {
            TopLevelFallback::Allowed => self.metadata.get(attribute_tag),
            TopLevelFallback::Disallowed => None,
        };

        if let Some(element) = per_frame {
            let mut duplicates = DuplicateSources::default();
            if shared.is_some() {
                duplicates = duplicates.with(FunctionalGroupSource::Shared);
            }
            if top_level_element.is_some() {
                duplicates = duplicates.with(FunctionalGroupSource::TopLevel);
            }
            return Ok(Some(ResolvedFunctionalGroup {
                element,
                source: per_frame_source,
                duplicate_sources: duplicates,
            }));
        }
        if let Some(element) = shared {
            let duplicates = if top_level_element.is_some() {
                DuplicateSources::default().with(FunctionalGroupSource::TopLevel)
            } else {
                DuplicateSources::default()
            };
            return Ok(Some(ResolvedFunctionalGroup {
                element,
                source: FunctionalGroupSource::Shared,
                duplicate_sources: duplicates,
            }));
        }
        Ok(top_level_element.map(|element| ResolvedFunctionalGroup {
            element,
            source: FunctionalGroupSource::TopLevel,
            duplicate_sources: DuplicateSources::default(),
        }))
    }
}

fn parse_frame_count(metadata: &MetadataSet) -> Result<usize, MultiframeError> {
    let Some(element) = metadata.get(NUMBER_OF_FRAMES) else {
        return Ok(1);
    };
    if element.vr() != VR::IS {
        return Err(MultiframeError::InvalidNumberOfFramesVr(element.vr()));
    }
    let MetadataValue::Text(values) = element.value() else {
        return match element.value() {
            MetadataValue::Empty => Err(MultiframeError::EmptyNumberOfFrames),
            _ => Err(MultiframeError::InvalidNumberOfFrames),
        };
    };
    if values.is_empty() {
        return Err(MultiframeError::EmptyNumberOfFrames);
    }
    if values.len() != 1 {
        return Err(MultiframeError::MultipleNumberOfFrames(values.len()));
    }
    let Some(raw_text) = values.first() else {
        return Err(MultiframeError::EmptyNumberOfFrames);
    };
    // DICOM PS3.5 section 6.2 counts retained padding in the 12-byte IS limit.
    if raw_text.len() > 12 {
        return Err(MultiframeError::InvalidNumberOfFrames);
    }
    let text = raw_text.trim_matches(' ');
    if text.is_empty() {
        return Err(MultiframeError::EmptyNumberOfFrames);
    }
    let parsed = text
        .parse::<i32>()
        .map_err(|_| MultiframeError::InvalidNumberOfFrames)?;
    if parsed == 0 {
        return Err(MultiframeError::ZeroNumberOfFrames);
    }
    if parsed < 0 {
        return Err(MultiframeError::InvalidNumberOfFrames);
    }
    usize::try_from(parsed).map_err(|_| MultiframeError::NumberOfFramesOverflow)
}

fn shared_item(metadata: &MetadataSet) -> Result<Option<&MetadataSet>, MultiframeError> {
    let Some(element) = metadata.get(SHARED_FUNCTIONAL_GROUPS) else {
        return Ok(None);
    };
    let MetadataValue::Sequence(items) = element.value() else {
        return Err(MultiframeError::InvalidSharedFunctionalGroups);
    };
    if element.vr() != VR::SQ {
        return Err(MultiframeError::InvalidSharedFunctionalGroups);
    }
    if items.len() > 1 {
        return Err(MultiframeError::SharedFunctionalGroupsCount(items.len()));
    }
    Ok(items.first())
}

fn per_frame_items(metadata: &MetadataSet) -> Result<Option<&[MetadataSet]>, MultiframeError> {
    let Some(element) = metadata.get(PER_FRAME_FUNCTIONAL_GROUPS) else {
        return Ok(None);
    };
    let MetadataValue::Sequence(items) = element.value() else {
        return Err(MultiframeError::InvalidPerFrameFunctionalGroups);
    };
    if element.vr() != VR::SQ {
        return Err(MultiframeError::InvalidPerFrameFunctionalGroups);
    }
    Ok(Some(items))
}

fn attribute_in_group(
    functional_groups: &MetadataSet,
    group_tag: Tag,
    attribute_tag: Tag,
    location: FunctionalGroupSource,
) -> Result<Option<&MetadataElement>, MultiframeError> {
    let Some(group) = functional_groups.get(group_tag) else {
        return Ok(None);
    };
    let MetadataValue::Sequence(items) = group.value() else {
        return Err(MultiframeError::InvalidFunctionalGroup {
            location,
            tag: group_tag,
        });
    };
    if group.vr() != VR::SQ || items.len() != 1 {
        return Err(MultiframeError::InvalidFunctionalGroup {
            location,
            tag: group_tag,
        });
    }
    Ok(items.first().and_then(|item| item.get(attribute_tag)))
}
