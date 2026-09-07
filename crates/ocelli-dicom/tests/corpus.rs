//! Local conformance evidence over the ignored DICOM corpus.
//!
//! The ordinary test suite remains self-contained. The `corpus` gate first
//! verifies every ignored file against `corpus/manifest.tsv`, then invokes
//! this ignored test. Failures report only row numbers and transfer-syntax
//! identifiers, never DICOM attribute values or instance identifiers.

use std::{fs, path::Path};

use ocelli_dicom::parse_part10;

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn every_manifest_row_parses_under_its_declared_transfer_syntax() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let data = root.join("corpus/data");
    let mut parsed_count = 0_usize;

    for (row_index, line) in manifest.lines().skip(1).enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let row = row_index + 2;
        let mut columns = line.split('\t');
        let relative_path = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no path"))?;
        let _modality = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no modality"))?;
        let declared_transfer_syntax = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no transfer syntax"))?
            .trim();
        if columns.count() != 6 {
            return Err(format!("manifest row {row} does not have nine columns"));
        }

        let bytes = fs::read(data.join(relative_path))
            .map_err(|_| format!("corpus file for manifest row {row} is unreadable"))?;
        let parsed = parse_part10(&bytes)
            .map_err(|_| format!("corpus manifest row {row} does not parse as Part 10"))?;
        let parsed_transfer_syntax = parsed.transfer_syntax().uid();
        if parsed_transfer_syntax != declared_transfer_syntax {
            return Err(format!(
                "manifest transfer syntax {declared_transfer_syntax} differs from parsed {parsed_transfer_syntax} at row {row}"
            ));
        }
        parsed_count += 1;
    }

    if parsed_count == 0 {
        return Err("the verified corpus contains no rows".to_owned());
    }

    Ok(())
}
