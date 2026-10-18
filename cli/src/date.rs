//! Calendar dates, which `check` needs for exemption expiry: today's, and a `--as-of` checked.
//!
//! Supplying the date is the host's job: the kernel has no clock on purpose (a wasm component has
//! nothing worth trusting, and an answer that depended on when it was asked would not be
//! reproducible). The conversions are Howard Hinnant's `civil_from_days` and `days_from_civil`,
//! which are the whole of what a calendar dependency would be used for here.

/// Today, UTC, as `YYYY-MM-DD`.
pub fn today() -> String {
  let days = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|since| since.as_secs() / 86_400)
    .unwrap_or(0) as i64;
  let (year, month, day) = civil_from_days(days);
  format!("{year:04}-{month:02}-{day:02}")
}

/// A `--as-of` value: a real calendar date, written `YYYY-MM-DD`.
pub fn parse(text: &str) -> Result<String, String> {
  let invalid = || format!("'{text}' is not a date written YYYY-MM-DD");
  let bytes = text.as_bytes();
  if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
    return Err(invalid());
  }
  let number = |range: std::ops::Range<usize>| {
    let digits = &text[range];
    digits
      .bytes()
      .all(|byte| byte.is_ascii_digit())
      .then(|| digits.parse::<i64>().ok())
      .flatten()
  };
  let (Some(year), Some(month), Some(day)) = (number(0..4), number(5..7), number(8..10)) else {
    return Err(invalid());
  };
  if !(1..=12).contains(&month) || day < 1 {
    return Err(invalid());
  }
  // A day past the end of its month rolls into the next one; a real date round-trips.
  let days = days_from_civil(year, month as u32, day as u32);
  if civil_from_days(days) != (year, month as u32, day as u32) {
    return Err(format!("{text} is not a day in the calendar"));
  }
  Ok(text.to_string())
}

/// Days since 1970-01-01 to a proleptic Gregorian date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
  let z = days + 719_468;
  let era = z.div_euclid(146_097);
  let doe = z.rem_euclid(146_097);
  let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
  let year = yoe + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
  let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
  (if month <= 2 { year + 1 } else { year }, month, day)
}

/// A proleptic Gregorian date to days since 1970-01-01.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
  let year = if month <= 2 { year - 1 } else { year };
  let era = year.div_euclid(400);
  let yoe = year.rem_euclid(400);
  let month = i64::from(month);
  let mp = if month > 2 { month - 3 } else { month + 9 };
  let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
  use super::*;
  use rstest::rstest;

  #[rstest]
  #[case(0, (1970, 1, 1))]
  #[case(1, (1970, 1, 2))]
  #[case(20_718, (2026, 9, 22))]
  // The month boundaries either side of a non-leap February, and the leap day itself.
  #[case(20_512, (2026, 2, 28))]
  #[case(20_513, (2026, 3, 1))]
  #[case(19_782, (2024, 2, 29))]
  #[case(19_783, (2024, 3, 1))]
  fn days_since_the_epoch_convert_to_the_date_they_are_and_back(
    #[case] days: i64,
    #[case] date: (i64, u32, u32),
  ) {
    assert_eq!(civil_from_days(days), date);
    assert_eq!(days_from_civil(date.0, date.1, date.2), days);
  }

  #[rstest]
  #[case("2026-09-22")]
  #[case("2024-02-29")]
  fn a_calendar_date_is_accepted(#[case] text: &str) {
    assert_eq!(parse(text), Ok(text.to_string()));
  }

  #[rstest]
  #[case("2026-9-22")]
  #[case("22/09/2026")]
  #[case("2026-13-01")]
  #[case("2026-00-10")]
  #[case("2026-02-29")]
  #[case("2026-04-31")]
  #[case("+026-01-01")]
  #[case("tomorrow")]
  fn anything_else_is_refused(#[case] text: &str) {
    assert!(parse(text).is_err(), "{text}");
  }
}
