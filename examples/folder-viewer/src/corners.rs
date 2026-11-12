//! Corner text for the folder viewer.
//!
//! Every derived corner string is made here and sent in the inspect JSON,
//! so the page places strings and never reformats them. Each function takes
//! the stored text already trimmed of its padding. The value rows are PS3.5
//! 6.2 Table 6.2-1 and the edge rule is PS3.3 C.7.6.1.1.1. The display
//! choices are `.claude/plans/F-212-design.md`.

/// A person name as `family, prefix given middle, suffix`, PS3.5 6.2 PN.
///
/// Only the first value of a multi-valued element is read. The alphabetic
/// group is used. When it has no component the ideographic group is used,
/// then the phonetic group. Empty components and their separators are dropped.
#[must_use]
pub fn person_name_text(stored: &str) -> String {
    let first = stored.split('\\').next().unwrap_or_default();
    for group in first.split('=').take(3) {
        let parts: Vec<&str> = group.split('^').map(str::trim).collect();
        let part = |index: usize| parts.get(index).copied().unwrap_or_default();
        let (family, given, middle, prefix, suffix) = (part(0), part(1), part(2), part(3), part(4));
        let names = [prefix, given, middle]
            .into_iter()
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let shown = [family, names.as_str(), suffix]
            .into_iter()
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        if !shown.is_empty() {
            return shown;
        }
    }
    String::new()
}

/// A date as `YYYY-MM-DD`, PS3.5 6.2 DA, or the stored text unchanged.
///
/// Only eight digits that name a real Gregorian day are rewritten. The
/// retired ACR-NEMA form `YYYY.MM.DD` is not parsed.
#[must_use]
pub fn dicom_date_text(stored: &str) -> String {
    match gregorian_date(stored) {
        Some(_) => format!(
            "{}-{}-{}",
            stored.get(0..4).unwrap_or_default(),
            stored.get(4..6).unwrap_or_default(),
            stored.get(6..8).unwrap_or_default()
        ),
        None => stored.to_owned(),
    }
}

/// A time as `HH`, `HH:MM` or `HH:MM:SS`, PS3.5 6.2 TM, or the stored text.
///
/// The fraction is dropped and never rounded, and a component the file
/// leaves out is not invented. The retired form `HH:MM:SS.frac` is not parsed.
#[must_use]
pub fn dicom_time_text(stored: &str) -> String {
    let (clock, fraction) = match stored.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (stored, None),
    };
    let fraction_ok = fraction.is_none_or(|digits| {
        (1..=6).contains(&digits.len()) && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    // A fraction belongs only after the seconds.
    let shape_ok = match clock.len() {
        2 | 4 => fraction.is_none(),
        6 => true,
        _ => false,
    };
    if !fraction_ok || !shape_ok || !clock.bytes().all(|byte| byte.is_ascii_digit()) {
        return stored.to_owned();
    }
    let limits = [23, 59, 60];
    let mut shown = Vec::new();
    for (index, limit) in limits.iter().enumerate() {
        let Some(pair) = clock.get(index * 2..index * 2 + 2) else {
            break;
        };
        match two_digits(pair) {
            Some(value) if value <= *limit => shown.push(pair),
            _ => return stored.to_owned(),
        }
    }
    shown.join(":")
}

/// The patient's age and where it came from, `PatientAge`, `computed` or empty.
///
/// A Patient's Age (0010,1010) present as `nnnD`, `nnnW`, `nnnM` or `nnnY`,
/// PS3.5 6.2 AS, is shown in words. One present in any other form gives
/// nothing, and no computed age replaces it. With no Patient's Age, two
/// valid DA dates with the study on or after the birth give the age at the
/// study. The clock is never read.
#[must_use]
pub fn age_text(
    patient_age: Option<&str>,
    birth: Option<&str>,
    study: Option<&str>,
) -> (String, &'static str) {
    if let Some(stored) = patient_age {
        return match stored_age(stored) {
            Some(text) => (text, "PatientAge"),
            None => (String::new(), ""),
        };
    }
    let (Some(birth), Some(study)) = (
        birth.and_then(gregorian_date),
        study.and_then(gregorian_date),
    ) else {
        return (String::new(), "");
    };
    match age_between(birth, study) {
        Some(text) => (text, "computed"),
        None => (String::new(), ""),
    }
}

/// The exposure in mAs as an exact decimal, and its source.
///
/// Exposure (0018,1152) when it is an IS in the PS3.5 6.2 range, source
/// `Exposure`. Otherwise Exposure in µAs (0018,1153) over 1000 in integer
/// arithmetic, source `ExposureInuAs`. Otherwise nothing. It is never
/// computed from tube current and exposure time.
#[must_use]
pub fn exposure_mas(exposure: Option<&str>, micro: Option<&str>) -> (Option<String>, &'static str) {
    if let Some(value) = exposure.and_then(integer_string) {
        return (Some(value.to_string()), "Exposure");
    }
    if let Some(value) = micro.and_then(integer_string) {
        let sign = if value < 0 { "-" } else { "" };
        let magnitude = value.unsigned_abs();
        let whole = magnitude / 1000;
        let thousandths = magnitude % 1000;
        let text = if thousandths == 0 {
            format!("{sign}{whole}")
        } else {
            let digits = format!("{thousandths:03}");
            format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
        };
        return (Some(text), "ExposureInuAs");
    }
    (None, "")
}

/// The one FD token, PS3.5 6.2 FD.
///
/// A finite value as `Display` writes it, which is the shortest form that
/// reads back as the same value and never an exponent. `None` for a
/// non-finite one, because PS3.5 6.2 permits NaN and infinity in an FD and
/// a corner line prints a quantity with a unit, the treatment `push_cine`
/// gives a non-finite Frame Time. There is no padding to trim, the same
/// reason the DS rule trims and this one does not.
#[must_use]
pub fn fd_text(value: f64) -> Option<String> {
    if value.is_finite() {
        Some(value.to_string())
    } else {
        None
    }
}

/// Left, right, top and bottom letters from Patient Orientation (0020,0020).
///
/// PS3.3 C.7.6.1.1.1. The first value is the direction of the rows and
/// names the right edge. The second is the direction of the columns and
/// names the bottom edge. Each opposite edge has each letter replaced by its
/// opposite. Exactly two values of one to three letters from `A P R L H F`,
/// with no letter together with its own opposite, are read. Anything else, the
/// quadruped terms included, is `None`.
#[must_use]
pub fn patient_orientation_letters(values: &[&str]) -> Option<[String; 4]> {
    let [row, column] = values else {
        return None;
    };
    let (row, column) = (row.trim(), column.trim());
    if !orientation_value(row) || !orientation_value(column) {
        return None;
    }
    Some([
        opposite_letters(row),
        row.to_owned(),
        opposite_letters(column),
        column.to_owned(),
    ])
}

fn orientation_value(value: &str) -> bool {
    (1..=3).contains(&value.len())
        && value
            .chars()
            .all(|letter| opposite_letter(letter).is_some())
        && !value
            .chars()
            .any(|letter| opposite_letter(letter).is_some_and(|other| value.contains(other)))
}

fn opposite_letter(letter: char) -> Option<char> {
    match letter {
        'A' => Some('P'),
        'P' => Some('A'),
        'R' => Some('L'),
        'L' => Some('R'),
        'H' => Some('F'),
        'F' => Some('H'),
        _ => None,
    }
}

fn opposite_letters(value: &str) -> String {
    value.chars().filter_map(opposite_letter).collect()
}

/// One calendar day, year, month and day.
type Day = (u32, u32, u32);

/// Eight ASCII digits naming a real Gregorian day, PS3.5 6.2 DA.
fn gregorian_date(stored: &str) -> Option<Day> {
    if stored.len() != 8 || !stored.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let year = stored.get(0..4)?.parse::<u32>().ok()?;
    let month = two_digits(stored.get(4..6)?)?;
    let day = two_digits(stored.get(6..8)?)?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn two_digits(text: &str) -> Option<u32> {
    if text.len() != 2 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A leap year is divisible by 4 and not by 100, or divisible by 400.
fn leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A day count on the proleptic Gregorian calendar, 1 on 1 January of year
/// 0. Only the difference of two counts is used.
fn day_number((year, month, day): Day) -> u64 {
    let mut days = 0_u64;
    for earlier in 0..year {
        days += if leap_year(earlier) { 366 } else { 365 };
    }
    for earlier in 1..month {
        days += u64::from(days_in_month(year, earlier));
    }
    days + u64::from(day)
}

/// `nnnD`, `nnnW`, `nnnM` or `nnnY` in words, PS3.5 6.2 AS.
fn stored_age(stored: &str) -> Option<String> {
    let digits = stored.get(0..3)?;
    if stored.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let count = digits.parse::<u32>().ok()?;
    let unit = match stored.get(3..4)? {
        "D" => "day",
        "W" => "week",
        "M" => "month",
        "Y" => "year",
        _ => return None,
    };
    Some(counted(count, unit))
}

/// The age at the study in its largest unit that is not 0.
///
/// Whole years are `sy - by`, less one while `(sm, sd)` is before
/// `(bm, bd)`. With 0 years, months are `(sy - by) * 12 + sm - bm`, less one
/// while `sd` is before `bd`. With 0 months, the Gregorian days between.
fn age_between(birth: Day, study: Day) -> Option<String> {
    if study < birth {
        return None;
    }
    let (by, bm, bd) = birth;
    let (sy, sm, sd) = study;
    let years = (sy - by) - u32::from((sm, sd) < (bm, bd));
    if years > 0 {
        return Some(counted(years, "year"));
    }
    // The study is not before the birth, so this sum is not negative.
    let months = ((sy - by) * 12 + sm)
        .checked_sub(bm)?
        .checked_sub(u32::from(sd < bd))?;
    if months > 0 {
        return Some(counted(months, "month"));
    }
    let days = day_number(study).checked_sub(day_number(birth))?;
    Some(format!("{days} {}", if days == 1 { "day" } else { "days" }))
}

fn counted(count: u32, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

/// An IS, PS3.5 6.2: an optional sign and decimal digits, at most 12
/// bytes, in the range -2^31 to 2^31 - 1.
fn integer_string(stored: &str) -> Option<i64> {
    let text = stored.trim_matches(' ');
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if text.len() > 12 || digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value = text.strip_prefix('+').unwrap_or(text).parse::<i64>().ok()?;
    i32::try_from(value).ok().map(i64::from)
}

#[cfg(test)]
mod fd_rule {
    use super::fd_text;

    /// PS3.3 2026c C.8.15.3.8 Table C.8-124 declares Exposure in mAs
    /// (0018,9332) FD and C.8.13.5.4 Table C.8-91 declares Effective Echo
    /// Time (0018,9082) FD. PS3.5 6.2 encodes FD as IEEE 754 binary64 with
    /// no padding to trim, and `Display` on a finite `f64` prints the
    /// shortest form that reads back as the same value and never an
    /// exponent. Each row is written as its stored bits, and the expected
    /// token is worked by hand from those two rules, never from a run of
    /// `fd_text`:
    ///
    /// 5.5 is 1.375 times 2^2, so the biased exponent is 1025 and the
    /// mantissa 0.375 is 0110 followed by zeros, `4016 0000 0000 0000`.
    /// 3.0 is 1.5 times 2^1, exponent 1024, mantissa 1000 followed by
    /// zeros, `4008 0000 0000 0000`. 12.25 is 1.53125 times 2^3, exponent
    /// 1026, mantissa 0.53125 is 10001 followed by zeros, `4028 8000 0000
    /// 0000`. 0.1, 0.123456789, 1e-7 and 1e21 are the plan's stored rows.
    /// 1e21 is 5^21 times 2^21 and 5^21 is below 2^53, so it is exact and
    /// prints as its 22 digits. Negative zero keeps its sign. The three
    /// non-finite rows are the PS3.5 sentence that all IEEE 754 values are
    /// permitted, and this plan shows a non-finite value as an absent
    /// field, the same treatment `push_cine` gives a non-finite Frame Time.
    #[test]
    fn fd_text_formats_one_finite_value_and_shows_no_non_finite_one() -> Result<(), String> {
        for (bits, want) in [
            (0x4016_0000_0000_0000, Some("5.5")),
            (0x4008_0000_0000_0000, Some("3")),
            (0x4028_8000_0000_0000, Some("12.25")),
            (0x3fb9_9999_9999_999a, Some("0.1")),
            (0x3fbf_9add_3739_635f, Some("0.123456789")),
            (0x3e7a_d7f2_9abc_af48, Some("0.0000001")),
            (0x444b_1ae4_d6e2_ef50, Some("1000000000000000000000")),
            (0x8000_0000_0000_0000, Some("-0")),
            (0x7ff0_0000_0000_0000, None),
            (0xfff0_0000_0000_0000, None),
            (0x7ff8_0000_0000_0000, None),
        ] {
            let got = fd_text(f64::from_bits(bits));
            let want = want.map(str::to_owned);
            if got != want {
                return Err(format!("{bits:#018x} gave {got:?}, want {want:?}"));
            }
        }
        Ok(())
    }
}
