//! The input contract, asserted on load.
//!
//! This module reads one side of a comparison: a directory holding a
//! `run.json`, one `<id>.raw` and one `<id>.json` per view. It refuses rather
//! than assumes, and every refusal here is a place where a later story could
//! otherwise break the comparator silently.
//!
//! Three of them are the load-bearing ones.
//!
//! **The view list is the union of the declared frame lists, and a `.raw` no
//! list names fails the run.** `run.json`'s `rows[]` stays stack-only on
//! purpose, because the accounting identity `readBack + unsupported ==
//! applicable` is asserted over it, so F-X007 put its reformats in
//! `volumes[].frames[]`. A comparator reading `rows[]` alone would compare 90
//! of 99 views and report success. The orphan refusal is what turns that into
//! a failure, and it keeps working when a later story adds a third list.
//!
//! **Each sidecar's `kind` is switched on and an unknown value is refused.**
//! Never defaulted to `stack`, because defaulting is how a new view kind gets
//! compared under the wrong rules.
//!
//! **The mapping from a manifest path to view identifiers is one to many.**
//! `real/mr_eay131/00000001.dcm` is one stack view and a member of a volume
//! whose three reformats are three more, so a census entry keyed on that path
//! resolves to four identifiers. Which of them it means is decided by the
//! entry's own `kind`, and an entry without one is refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use thiserror::Error;

use crate::frame::{Frame, FrameError};
use crate::geometry::Camera;

/// The two values `kind` may take today. F-X007 declares both and
/// `check_sidecars.py` refuses a sidecar carrying neither.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ViewKind {
    Stack,
    VolumeReformat,
}

impl ViewKind {
    /// # Errors
    /// On any value but the two declared ones. Not defaulted, ever.
    pub fn parse(raw: &str) -> Result<Self, LoadError> {
        match raw {
            "stack" => Ok(Self::Stack),
            "volume-reformat" => Ok(Self::VolumeReformat),
            other => Err(LoadError::UnknownKind(other.to_owned())),
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Stack => "stack",
            Self::VolumeReformat => "volume-reformat",
        }
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("{0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("{0}: {1}")]
    Json(PathBuf, serde_json::Error),
    #[error(
        "sidecar kind {0:?} is not one this comparator knows. A new view kind \
         is refused rather than defaulted to `stack`, because defaulting is how \
         a new kind gets compared under the wrong rules. See the output \
         contract in docs/lld/oracle.md"
    )]
    UnknownKind(String),
    #[error(
        "{0} is present in the directory and no declared frame list names it. \
         run.json's rows[] is stack-only and the volume frames are in \
         volumes[].frames[], so a comparator reading one list would compare \
         part of the run and report success. Every `.raw` must be declared"
    )]
    OrphanRaw(String),
    #[error("{0} is declared in run.json and its {1} is not in the directory")]
    DeclaredFileMissing(String, &'static str),
    #[error("{0} is declared twice, by two different frame lists")]
    DuplicateDeclaration(String),
    #[error("{path}: {field} is missing or is not {expected}")]
    Field {
        path: String,
        field: String,
        expected: &'static str,
    },
    #[error(
        "{id}: the frame on disk hashes to {actual} and its sidecar declares \
         {declared}. The reference half hashes every frame twice already, so a \
         third hash at read is what stops a file edited after the run being \
         compared as though somebody had rendered it"
    )]
    DigestMismatch {
        id: String,
        declared: String,
        actual: String,
    },
    #[error(
        "a census entry names {0:?} and no declared view of kind {1} resolves \
         from it"
    )]
    UnresolvedCensusEntry(String, &'static str),
    #[error(
        "a census entry carries neither `id` nor `path`, so nothing says which \
         view it means"
    )]
    CensusEntryWithoutKey,
    #[error("{0}")]
    Frame(#[from] FrameError),
}

fn read_json(path: &Path) -> Result<Value, LoadError> {
    let text = fs::read_to_string(path).map_err(|error| LoadError::Io(path.to_owned(), error))?;
    serde_json::from_str(&text).map_err(|error| LoadError::Json(path.to_owned(), error))
}

/// RFC 6901 pointer lookup, with the field name in the error rather than a
/// bare `None`.
///
/// # Errors
/// When the pointer resolves to nothing.
pub fn require<'a>(root: &'a Value, pointer: &str, owner: &str) -> Result<&'a Value, LoadError> {
    root.pointer(pointer).ok_or_else(|| LoadError::Field {
        path: owner.to_owned(),
        field: pointer.to_owned(),
        expected: "present",
    })
}

fn require_str(root: &Value, pointer: &str, owner: &str) -> Result<String, LoadError> {
    require(root, pointer, owner)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "a string",
        })
}

fn require_u32(root: &Value, pointer: &str, owner: &str) -> Result<u32, LoadError> {
    let raw = require(root, pointer, owner)?
        .as_u64()
        .ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "a non-negative integer",
        })?;
    u32::try_from(raw).map_err(|_| LoadError::Field {
        path: owner.to_owned(),
        field: pointer.to_owned(),
        expected: "an integer that fits a u32",
    })
}

fn require_f64(root: &Value, pointer: &str, owner: &str) -> Result<f64, LoadError> {
    require(root, pointer, owner)?
        .as_f64()
        .ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "a number",
        })
}

fn require_triple(root: &Value, pointer: &str, owner: &str) -> Result<[f64; 3], LoadError> {
    let list = require(root, pointer, owner)?
        .as_array()
        .ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "an array",
        })?;
    let mut out = [0.0_f64; 3];
    if list.len() != 3 {
        return Err(LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "an array of three numbers",
        });
    }
    for (slot, value) in out.iter_mut().zip(list.iter()) {
        *slot = value.as_f64().ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "an array of three numbers",
        })?;
    }
    Ok(out)
}

/// One view's sidecar, kept as parsed JSON so the declared field lists in
/// `attribution.rs` can address it by pointer and a field nobody compares is
/// still carried into the record.
#[derive(Clone, Debug)]
pub struct Sidecar {
    pub id: String,
    pub kind: ViewKind,
    pub json: Value,
}

impl Sidecar {
    /// # Errors
    /// When `frame.width` or `frame.height` is absent or not an integer.
    pub fn frame_size(&self) -> Result<(u32, u32), LoadError> {
        Ok((
            require_u32(&self.json, "/frame/width", &self.id)?,
            require_u32(&self.json, "/frame/height", &self.id)?,
        ))
    }

    /// # Errors
    /// When `frame.sha256` is absent or not a string.
    pub fn declared_digest(&self) -> Result<String, LoadError> {
        require_str(&self.json, "/frame/sha256", &self.id)
    }

    /// # Errors
    /// When a camera field is absent or malformed.
    pub fn camera(&self) -> Result<Camera, LoadError> {
        let normal = match self.json.pointer("/camera/viewPlaneNormal") {
            Some(_) => Some(require_triple(
                &self.json,
                "/camera/viewPlaneNormal",
                &self.id,
            )?),
            None => None,
        };
        Ok(Camera {
            position: require_triple(&self.json, "/camera/position", &self.id)?,
            focal_point: require_triple(&self.json, "/camera/focalPoint", &self.id)?,
            view_up: require_triple(&self.json, "/camera/viewUp", &self.id)?,
            parallel_scale: require_f64(&self.json, "/camera/parallelScale", &self.id)?,
            view_plane_normal: normal,
        })
    }

    /// The stack frame's published canvas scale, vertical then horizontal.
    ///
    /// F-X007 records this on every stack row rather than only on the two the
    /// run lists under `downsampled`, so this crate reads it instead of
    /// holding a second copy of `canvasScale`.
    ///
    /// # Errors
    /// When either component is absent.
    pub fn canvas_pixels_per_source_pixel(&self) -> Result<(f64, f64), LoadError> {
        Ok((
            require_f64(&self.json, "/canvasPixelsPerSourcePixel/vertical", &self.id)?,
            require_f64(
                &self.json,
                "/canvasPixelsPerSourcePixel/horizontal",
                &self.id,
            )?,
        ))
    }

    /// The source image's pixel grid, rows then columns.
    ///
    /// # Errors
    /// When either is absent.
    pub fn source_grid(&self) -> Result<(u32, u32), LoadError> {
        Ok((
            require_u32(&self.json, "/image/rows", &self.id)?,
            require_u32(&self.json, "/image/columns", &self.id)?,
        ))
    }

    /// A reformat's canvas scale in millimetres.
    ///
    /// # Errors
    /// When it is absent.
    pub fn millimetres_per_canvas_pixel(&self) -> Result<f64, LoadError> {
        require_f64(&self.json, "/reformat/millimetresPerCanvasPixel", &self.id)
    }
}

/// One declared view, before its files are read.
#[derive(Clone, Debug)]
pub struct DeclaredView {
    pub id: String,
    pub kind: ViewKind,
    /// The manifest path for a stack view. A reformat has none, because it is
    /// a view of a series and not of an instance.
    pub path: Option<String>,
    /// The subject id for a reformat, so the run's `volumes[]` entry that
    /// explains it can be found without re-deriving the identifier scheme.
    pub subject: Option<String>,
}

/// One side of a comparison: one directory of reference-half output.
#[derive(Clone, Debug)]
pub struct Run {
    pub directory: PathBuf,
    pub json: Value,
    pub views: BTreeMap<String, DeclaredView>,
    pub sidecars: BTreeMap<String, Sidecar>,
    /// Every `<id>` for which a `.raw` exists on disk.
    pub raw_present: BTreeSet<String>,
    /// Manifest path to the view identifiers it produces. One to many.
    pub by_path: BTreeMap<String, BTreeSet<String>>,
    /// Category tokens per view, resolved from the manifest tokens the stack
    /// sidecars carry.
    pub categories: BTreeMap<String, Vec<String>>,
}

impl Run {
    /// Read a run directory and assert the input contract over it.
    ///
    /// # Errors
    /// On any refusal in `LoadError`.
    pub fn load(directory: &Path) -> Result<Self, LoadError> {
        let run_path = directory.join("run.json");
        let json = read_json(&run_path)?;

        let mut views: BTreeMap<String, DeclaredView> = BTreeMap::new();
        let mut by_path: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        // Declared list one: rows[], stack only, and only the rows that
        // produced a frame. An entry with ok false is a row the reference
        // could not render and `unsupported.json` already accounts for it.
        let rows = json
            .pointer("/rows")
            .and_then(Value::as_array)
            .ok_or_else(|| LoadError::Field {
                path: "run.json".to_owned(),
                field: "/rows".to_owned(),
                expected: "an array",
            })?;
        for row in rows {
            let kind = ViewKind::parse(&require_str(row, "/kind", "run.json rows[]")?)?;
            if row.pointer("/ok").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let id = require_str(row, "/id", "run.json rows[]")?;
            let path = require_str(row, "/path", "run.json rows[]")?;
            if views.contains_key(&id) {
                return Err(LoadError::DuplicateDeclaration(id));
            }
            by_path.entry(path.clone()).or_default().insert(id.clone());
            views.insert(
                id.clone(),
                DeclaredView {
                    id,
                    kind,
                    path: Some(path),
                    subject: None,
                },
            );
        }

        // Declared list two: volumes[].frames[]. A subject refused at a
        // boundary carries an empty frame list, which is how
        // `volume__real__ct_cmb_mml` declares that it produced no reference
        // output rather than leaving nine frames unaccounted for.
        if let Some(volumes) = json.pointer("/volumes").and_then(Value::as_array) {
            for volume in volumes {
                let subject = require_str(volume, "/id", "run.json volumes[]")?;
                let Some(frames) = volume.pointer("/frames").and_then(Value::as_array) else {
                    continue;
                };
                let member_paths = volume_member_paths(volume);
                for frame in frames {
                    let id = require_str(frame, "/id", "run.json volumes[].frames[]")?;
                    if views.contains_key(&id) {
                        return Err(LoadError::DuplicateDeclaration(id));
                    }
                    for member in &member_paths {
                        by_path
                            .entry(member.clone())
                            .or_default()
                            .insert(id.clone());
                    }
                    views.insert(
                        id.clone(),
                        DeclaredView {
                            id,
                            kind: ViewKind::VolumeReformat,
                            path: None,
                            subject: Some(subject.clone()),
                        },
                    );
                }
            }
        }

        let raw_present = raw_ids_on_disk(directory)?;

        let mut sidecars = BTreeMap::new();
        for view in views.values() {
            let sidecar_path = directory.join(format!("{}.json", view.id));
            if !sidecar_path.is_file() {
                return Err(LoadError::DeclaredFileMissing(view.id.clone(), "sidecar"));
            }
            if !raw_present.contains(&view.id) {
                return Err(LoadError::DeclaredFileMissing(view.id.clone(), "`.raw`"));
            }
            let json = read_json(&sidecar_path)?;
            let kind = ViewKind::parse(&require_str(&json, "/kind", &view.id)?)?;
            sidecars.insert(
                view.id.clone(),
                Sidecar {
                    id: view.id.clone(),
                    kind,
                    json,
                },
            );
        }

        let mut run = Self {
            directory: directory.to_owned(),
            json,
            views,
            sidecars,
            raw_present,
            by_path,
            categories: BTreeMap::new(),
        };
        run.categories = run.resolve_categories()?;
        run.verify_structure()?;
        Ok(run)
    }

    /// Every structural claim the loader makes, re-runnable.
    ///
    /// It is a separate function rather than inline in `load` so the mutation
    /// catalogue can break one of these claims in memory and watch the same
    /// code refuse it. A guard nobody has watched fail is not a guard.
    ///
    /// # Errors
    /// On an orphan `.raw`, a declared view with no files, an unknown kind, or
    /// a sidecar whose kind disagrees with its declaration.
    pub fn verify_structure(&self) -> Result<(), LoadError> {
        for id in &self.raw_present {
            if !self.views.contains_key(id) {
                return Err(LoadError::OrphanRaw(format!("{id}.raw")));
            }
        }
        for view in self.views.values() {
            let sidecar = self
                .sidecars
                .get(&view.id)
                .ok_or_else(|| LoadError::DeclaredFileMissing(view.id.clone(), "sidecar"))?;
            // Re-parsed from the JSON rather than read from the field, so a
            // sidecar edited after load is refused by the same code that
            // refuses one edited before it.
            let declared = ViewKind::parse(&require_str(&sidecar.json, "/kind", &view.id)?)?;
            if declared != view.kind || sidecar.kind != view.kind {
                return Err(LoadError::UnknownKind(format!(
                    "{}: run.json declares {} and the sidecar says {}",
                    view.id,
                    view.kind.label(),
                    sidecar.kind.label()
                )));
            }
            if !self.raw_present.contains(&view.id) {
                return Err(LoadError::DeclaredFileMissing(view.id.clone(), "`.raw`"));
            }
        }
        Ok(())
    }

    /// Every top-level key of `run.json` whose name ends in `Sha256`.
    ///
    /// Collected by suffix rather than from a fixed list, because F-X007 added
    /// `volumeParamsSha256` and `volumeTruthSha256` and a fixed list would
    /// have silently stopped covering the inputs that decide the frames.
    #[must_use]
    pub fn digests(&self) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        if let Some(object) = self.json.as_object() {
            for (key, value) in object {
                if key.ends_with("Sha256")
                    && let Some(text) = value.as_str()
                {
                    found.insert(key.clone(), text.to_owned());
                }
            }
        }
        found
    }

    /// Read one view's frame, checking its length and its declared digest.
    ///
    /// # Errors
    /// When the file is unreadable, the wrong length, or hashes to something
    /// other than what its sidecar declares.
    pub fn read_frame(&self, id: &str) -> Result<Frame, LoadError> {
        let sidecar = self
            .sidecars
            .get(id)
            .ok_or_else(|| LoadError::DeclaredFileMissing(id.to_owned(), "sidecar"))?;
        let (width, height) = sidecar.frame_size()?;
        let path = self.directory.join(format!("{id}.raw"));
        let bytes = fs::read(&path).map_err(|error| LoadError::Io(path.clone(), error))?;
        let frame = Frame::new(width, height, bytes)?;
        let declared = sidecar.declared_digest()?;
        let actual = frame.sha256_hex();
        if actual != declared {
            return Err(LoadError::DigestMismatch {
                id: id.to_owned(),
                declared,
                actual,
            });
        }
        Ok(frame)
    }

    /// The view identifiers a census entry names.
    ///
    /// An entry carries `id` for a volume frame and `path` for a stack row,
    /// and both are accepted. A `path` resolves through the one-to-many map
    /// and is then FILTERED by the entry's own `kind`, because a series member
    /// path names one stack view and three reformats and an entry means only
    /// one of those kinds.
    ///
    /// # Errors
    /// When the entry has no key, carries an unknown kind, or names nothing.
    pub fn resolve_census_entry(
        &self,
        entry: &Value,
        default_kind: Option<ViewKind>,
    ) -> Result<BTreeSet<String>, LoadError> {
        let kind = match entry.pointer("/kind").and_then(Value::as_str) {
            Some(raw) => ViewKind::parse(raw)?,
            None => default_kind.ok_or_else(|| LoadError::UnknownKind("absent".to_owned()))?,
        };
        if let Some(id) = entry.pointer("/id").and_then(Value::as_str) {
            if self.views.contains_key(id) {
                return Ok(BTreeSet::from([id.to_owned()]));
            }
            return Err(LoadError::UnresolvedCensusEntry(
                id.to_owned(),
                kind.label(),
            ));
        }
        let Some(path) = entry.pointer("/path").and_then(Value::as_str) else {
            return Err(LoadError::CensusEntryWithoutKey);
        };
        let found: BTreeSet<String> = self
            .by_path
            .get(path)
            .map(|ids| {
                ids.iter()
                    .filter(|id| self.views.get(*id).is_some_and(|view| view.kind == kind))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if found.is_empty() {
            return Err(LoadError::UnresolvedCensusEntry(
                path.to_owned(),
                kind.label(),
            ));
        }
        Ok(found)
    }

    /// The views `run.json` lists under `lowInformation`.
    ///
    /// Read from the run record rather than rederived from the frames, because
    /// the reference half already computes saturation against a declared
    /// threshold in `render-params.json` and a second derivation would be free
    /// to drift from the first.
    ///
    /// # Errors
    /// When an entry names nothing.
    pub fn low_information(&self) -> Result<BTreeSet<String>, LoadError> {
        let mut found = BTreeSet::new();
        let Some(rows) = self
            .json
            .pointer("/lowInformation/rows")
            .and_then(Value::as_array)
        else {
            return Ok(found);
        };
        for row in rows {
            found.extend(self.resolve_census_entry(row, None)?);
        }
        Ok(found)
    }

    /// The views `run.json` lists under `downsampled`.
    ///
    /// `downsampled` is a stack concept: a reformat plane has no source pixel
    /// grid to be a magnification of. So the entries carry no `kind` and the
    /// default is supplied here rather than being guessed per entry.
    ///
    /// # Errors
    /// When an entry names nothing.
    pub fn downsampled(&self) -> Result<BTreeSet<String>, LoadError> {
        let mut found = BTreeSet::new();
        let Some(rows) = self.json.pointer("/downsampled").and_then(Value::as_array) else {
            return Ok(found);
        };
        for row in rows {
            found.extend(self.resolve_census_entry(row, Some(ViewKind::Stack))?);
        }
        Ok(found)
    }

    /// F-X007's per-subject measurement of where the reference's volume
    /// geometry departs from a truth the corpus generator knows by
    /// construction. Rung 3 of the attribution ladder reads it, so a volume
    /// view whose geometry the reference got wrong is attributed without a
    /// hand-written register entry.
    #[must_use]
    pub fn reference_divergence_for(&self, id: &str) -> Option<&Value> {
        let subject = self.views.get(id)?.subject.as_ref()?;
        let volumes = self.json.pointer("/volumes")?.as_array()?;
        for volume in volumes {
            if volume.pointer("/id").and_then(Value::as_str) == Some(subject.as_str()) {
                return match volume.pointer("/referenceDivergence") {
                    Some(Value::Null) | None => None,
                    Some(value) => Some(value),
                };
            }
        }
        None
    }

    /// The manifest category tokens for every view.
    ///
    /// A stack sidecar carries them under `row.categories`. **A volume sidecar
    /// carries no `row` block at all**, which the design plan assumed it would,
    /// so a reformat's class is resolved from its members' own stack sidecars.
    /// The class token is a property of the pixel data and every member of a
    /// series carries the same one, so a subject whose members disagree is
    /// refused rather than resolved from the first.
    fn resolve_categories(&self) -> Result<BTreeMap<String, Vec<String>>, LoadError> {
        let mut found = BTreeMap::new();
        for (id, sidecar) in &self.sidecars {
            let tokens = match sidecar.kind {
                ViewKind::Stack => string_list(&sidecar.json, "/row/categories", id)?,
                ViewKind::VolumeReformat => self.volume_categories(id)?,
            };
            found.insert(id.clone(), tokens);
        }
        Ok(found)
    }

    fn volume_categories(&self, id: &str) -> Result<Vec<String>, LoadError> {
        let sidecar = self
            .sidecars
            .get(id)
            .ok_or_else(|| LoadError::DeclaredFileMissing(id.to_owned(), "sidecar"))?;
        let members = require(&sidecar.json, "/volume/members", id)?
            .as_array()
            .ok_or_else(|| LoadError::Field {
                path: id.to_owned(),
                field: "/volume/members".to_owned(),
                expected: "an array",
            })?;
        let mut union: BTreeSet<String> = BTreeSet::new();
        for member in members {
            let member_sidecar = require_str(member, "/stackSidecar", id)?;
            let stem = member_sidecar
                .strip_suffix(".json")
                .unwrap_or(&member_sidecar);
            let stack = self
                .sidecars
                .get(stem)
                .ok_or_else(|| LoadError::DeclaredFileMissing(stem.to_owned(), "sidecar"))?;
            union.extend(string_list(&stack.json, "/row/categories", stem)?);
        }
        Ok(union.into_iter().collect())
    }
}

fn string_list(root: &Value, pointer: &str, owner: &str) -> Result<Vec<String>, LoadError> {
    let list = require(root, pointer, owner)?
        .as_array()
        .ok_or_else(|| LoadError::Field {
            path: owner.to_owned(),
            field: pointer.to_owned(),
            expected: "an array",
        })?;
    list.iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| LoadError::Field {
                    path: owner.to_owned(),
                    field: pointer.to_owned(),
                    expected: "an array of strings",
                })
        })
        .collect()
}

fn volume_member_paths(volume: &Value) -> Vec<String> {
    volume
        .pointer("/members")
        .and_then(Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(|member| member.pointer("/path").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn raw_ids_on_disk(directory: &Path) -> Result<BTreeSet<String>, LoadError> {
    let mut found = BTreeSet::new();
    let entries =
        fs::read_dir(directory).map_err(|error| LoadError::Io(directory.to_owned(), error))?;
    for entry in entries {
        let entry = entry.map_err(|error| LoadError::Io(directory.to_owned(), error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".raw") {
            found.insert(stem.to_owned());
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::json;

    use super::{DeclaredView, LoadError, Run, ViewKind};

    /// A synthetic run whose one manifest path yields FOUR view identifiers:
    /// the instance's own stack frame and the three reformats of the volume it
    /// is a member of.
    fn synthetic() -> Run {
        let mut views = BTreeMap::new();
        let mut by_path: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        views.insert(
            "series__slice_000".to_owned(),
            DeclaredView {
                id: "series__slice_000".to_owned(),
                kind: ViewKind::Stack,
                path: Some("series/slice_000.dcm".to_owned()),
                subject: None,
            },
        );
        by_path
            .entry("series/slice_000.dcm".to_owned())
            .or_default()
            .insert("series__slice_000".to_owned());
        for orientation in ["AXIAL", "SAGITTAL", "CORONAL"] {
            let id = format!("volume__series__{orientation}");
            views.insert(
                id.clone(),
                DeclaredView {
                    id: id.clone(),
                    kind: ViewKind::VolumeReformat,
                    path: None,
                    subject: Some("volume__series".to_owned()),
                },
            );
            by_path
                .entry("series/slice_000.dcm".to_owned())
                .or_default()
                .insert(id);
        }
        Run {
            directory: std::path::PathBuf::from("."),
            json: json!({ "rows": [], "volumes": [] }),
            views,
            sidecars: BTreeMap::new(),
            raw_present: BTreeSet::new(),
            by_path,
            categories: BTreeMap::new(),
        }
    }

    /// The plan's F-X007 guard, written before F-X007 existed and kept now
    /// that it has landed. One manifest path, four views.
    #[test]
    fn a_manifest_path_resolves_to_many_view_identifiers() {
        let run = synthetic();
        let Some(all) = run.by_path.get("series/slice_000.dcm") else {
            assert!(core::hint::black_box(false), "the path resolved to nothing");
            return;
        };
        assert_eq!(all.len(), 4);
    }

    /// A census entry keyed on a path is filtered by its own `kind`, so the
    /// stack entry does not silently mark the three reformats as well.
    #[test]
    fn a_path_keyed_entry_is_filtered_by_its_kind() {
        let run = synthetic();
        let stack = run.resolve_census_entry(
            &json!({ "kind": "stack", "path": "series/slice_000.dcm" }),
            None,
        );
        assert_eq!(stack.ok().map(|set| set.len()), Some(1));
        let reformats = run.resolve_census_entry(
            &json!({ "kind": "volume-reformat", "path": "series/slice_000.dcm" }),
            None,
        );
        assert_eq!(reformats.ok().map(|set| set.len()), Some(3));
    }

    /// A census entry keyed on `id` resolves as well as one keyed on `path`.
    #[test]
    fn an_id_keyed_entry_resolves() {
        let run = synthetic();
        let found = run.resolve_census_entry(
            &json!({ "kind": "volume-reformat", "id": "volume__series__SAGITTAL" }),
            None,
        );
        assert_eq!(found.ok().map(|set| set.len()), Some(1));
    }

    #[test]
    fn a_census_entry_with_no_key_is_refused() {
        let run = synthetic();
        assert!(matches!(
            run.resolve_census_entry(&json!({ "kind": "stack" }), None),
            Err(LoadError::CensusEntryWithoutKey)
        ));
    }

    #[test]
    fn a_census_entry_naming_nothing_is_refused() {
        let run = synthetic();
        assert!(matches!(
            run.resolve_census_entry(&json!({ "kind": "stack", "path": "no/such.dcm" }), None),
            Err(LoadError::UnresolvedCensusEntry(_, _))
        ));
    }

    /// An unknown `kind` is refused rather than defaulted to `stack`.
    #[test]
    fn an_unknown_kind_is_refused() {
        assert!(matches!(
            ViewKind::parse("volume"),
            Err(LoadError::UnknownKind(_))
        ));
        assert!(matches!(
            ViewKind::parse(""),
            Err(LoadError::UnknownKind(_))
        ));
        assert_eq!(ViewKind::parse("stack").ok(), Some(ViewKind::Stack));
        assert_eq!(
            ViewKind::parse("volume-reformat").ok(),
            Some(ViewKind::VolumeReformat)
        );
    }

    /// A `.raw` in the directory that no declared frame list names fails the
    /// run. This is the guard that stops a partial comparison reporting as a
    /// complete one.
    #[test]
    fn an_undeclared_raw_fails_the_run() {
        let mut run = synthetic();
        run.raw_present.insert("nobody__declared_me".to_owned());
        assert!(matches!(
            run.verify_structure(),
            Err(LoadError::OrphanRaw(_))
        ));
    }

    /// Every top-level `*Sha256` key is collected, including the two F-X007
    /// added, and nothing else is.
    #[test]
    fn every_sha256_key_is_collected_by_suffix() {
        let mut run = synthetic();
        run.json = json!({
            "manifestSha256": "a",
            "renderParamsSha256": "b",
            "unsupportedSha256": "c",
            "volumeParamsSha256": "d",
            "volumeTruthSha256": "e",
            "packages": { "core": "5.8.2" },
            "story": "F-010, F-X007"
        });
        let digests = run.digests();
        assert_eq!(digests.len(), 5);
        assert!(digests.contains_key("volumeTruthSha256"));
        assert!(!digests.contains_key("packages"));
    }
}
