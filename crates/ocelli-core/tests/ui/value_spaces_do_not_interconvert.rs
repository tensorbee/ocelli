//! HLD section 16.1's claim: "You cannot accidentally window a stored value."
//!
//! `Stored`, `Modality` and `Display` are three distinct newtypes over `f32`
//! with no conversion between them, because the modality LUT and the VOI LUT
//! are arithmetic that section 18 requires to exist exactly once, in
//! `ocelli-pixel`. A `From` here would be a second copy of a LUT stage.
//!
//! Nothing that runs can observe that, in the same way nothing that runs can
//! observe the space parameters on `Pt`. This case is the guard: if anyone
//! adds a conversion, or collapses the three into one type alias, it stops
//! failing to compile and the suite says so.

use ocelli_core::{Display, Modality, Stored};

fn main() {
    // A stored value is not a modality value. Skipping the modality LUT has
    // to be a compile error, not a silent identity.
    let _wrong: Modality = Stored(1.0);
    // And a modality value is not a display value. Skipping the VOI LUT is
    // the same defect one stage later.
    let _also_wrong: Display = Modality(1.0);
}
