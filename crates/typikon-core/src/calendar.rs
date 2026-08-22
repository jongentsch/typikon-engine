use chrono::{Datelike, Duration, NaiveDate};
use thiserror::Error;
use typikon_schema::FixedCalendar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl std::fmt::Display for CalendarDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneComputation {
    pub tone: String,
    pub ordinal: u8,
    pub anchor: NaiveDate,
    pub weeks_from_anchor: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CalendarError {
    #[error("calendar calculation does not support year {0}")]
    UnsupportedYear(i32),
    #[error("Revised Julian projection currently supports years 1600 through 9999")]
    RevisedJulianRange,
    #[error("the Octoechos tone cycle requires exactly eight pack tone names")]
    InvalidToneVocabulary,
    #[error("the ordinary Octoechos tone cycle is suspended for {date}")]
    ToneCycleSuspended { date: NaiveDate },
    #[error("calendar arithmetic overflowed")]
    Overflow,
}

/// Projects a Gregorian civil date into the selected fixed calendar.
///
/// # Errors
///
/// Returns an error when the date is outside the implemented Revised Julian
/// range or the arithmetic cannot be represented.
pub fn project_fixed_date(
    date: NaiveDate,
    calendar: FixedCalendar,
) -> Result<CalendarDate, CalendarError> {
    match calendar {
        FixedCalendar::Gregorian => Ok(CalendarDate {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }),
        FixedCalendar::Julian => Ok(julian_from_jdn(gregorian_jdn(date))),
        FixedCalendar::RevisedJulian => {
            if !(1600..=9999).contains(&date.year()) {
                return Err(CalendarError::RevisedJulianRange);
            }
            revised_julian_from_gregorian(date)
        }
    }
}

/// Calculates Orthodox Pascha using the Julian ecclesiastical computus.
///
/// # Errors
///
/// Returns an error for years outside `1..=9999` or arithmetic that cannot be
/// represented by `chrono`.
pub fn orthodox_pascha(year: i32) -> Result<NaiveDate, CalendarError> {
    if !(1..=9999).contains(&year) {
        return Err(CalendarError::UnsupportedYear(year));
    }
    let year_mod_four = year.rem_euclid(4);
    let year_mod_seven = year.rem_euclid(7);
    let year_mod_nineteen = year.rem_euclid(19);
    let lunar_offset = (19 * year_mod_nineteen + 15).rem_euclid(30);
    let weekday_offset = (2 * year_mod_four + 4 * year_mod_seven - lunar_offset + 34).rem_euclid(7);
    let value = lunar_offset + weekday_offset + 114;
    let month = u32::try_from(value / 31).map_err(|_| CalendarError::Overflow)?;
    let day = u32::try_from(value.rem_euclid(31) + 1).map_err(|_| CalendarError::Overflow)?;
    let jdn = julian_jdn(year, month, day);
    gregorian_from_jdn(jdn)
}

/// Calculates the ordinary Octoechos tone for a civil liturgical date.
///
/// # Errors
///
/// Returns an error unless the pack supplies exactly eight tones, when the
/// date is Pascha through Bright Saturday, or when date arithmetic overflows.
pub fn octoechos_tone(date: NaiveDate, tones: &[String]) -> Result<ToneComputation, CalendarError> {
    if tones.len() != 8 {
        return Err(CalendarError::InvalidToneVocabulary);
    }
    let pascha = orthodox_pascha(date.year())?;
    let anchor = pascha
        .checked_add_signed(Duration::days(7))
        .ok_or(CalendarError::Overflow)?;
    if date >= pascha && date < anchor {
        return Err(CalendarError::ToneCycleSuspended { date });
    }
    let anchor = if date >= anchor {
        anchor
    } else {
        orthodox_pascha(date.year() - 1)?
            .checked_add_signed(Duration::days(7))
            .ok_or(CalendarError::Overflow)?
    };
    let weeks_from_anchor = (date - anchor).num_days().div_euclid(7);
    let zero_based =
        usize::try_from(weeks_from_anchor.rem_euclid(8)).map_err(|_| CalendarError::Overflow)?;
    Ok(ToneComputation {
        tone: tones[zero_based].clone(),
        ordinal: u8::try_from(zero_based + 1).map_err(|_| CalendarError::Overflow)?,
        anchor,
        weeks_from_anchor,
    })
}

fn gregorian_jdn(date: NaiveDate) -> i64 {
    let month = i64::from(date.month());
    let day = i64::from(date.day());
    let a = (14 - month) / 12;
    let year = i64::from(date.year()) + 4800 - a;
    let month = month + 12 * a - 3;
    day + (153 * month + 2) / 5 + 365 * year + year / 4 - year / 100 + year / 400 - 32_045
}

fn julian_jdn(year: i32, month: u32, day: u32) -> i64 {
    let month = i64::from(month);
    let day = i64::from(day);
    let a = (14 - month) / 12;
    let year = i64::from(year) + 4800 - a;
    let month = month + 12 * a - 3;
    day + (153 * month + 2) / 5 + 365 * year + year / 4 - 32_083
}

fn julian_from_jdn(jdn: i64) -> CalendarDate {
    let c = jdn + 32_082;
    let d = (4 * c + 3) / 1_461;
    let e = c - (1_461 * d) / 4;
    let m = (5 * e + 2) / 153;
    CalendarDate {
        day: u32::try_from(e - (153 * m + 2) / 5 + 1).expect("JDN day is in range"),
        month: u32::try_from(m + 3 - 12 * (m / 10)).expect("JDN month is in range"),
        year: i32::try_from(d - 4800 + m / 10).expect("JDN year is in range"),
    }
}

fn gregorian_from_jdn(jdn: i64) -> Result<NaiveDate, CalendarError> {
    let shifted = jdn + 32_044;
    let century = (4 * shifted + 3) / 146_097;
    let remaining = shifted - (146_097 * century) / 4;
    let quadrennial = (4 * remaining + 3) / 1_461;
    let day_in_cycle = remaining - (1_461 * quadrennial) / 4;
    let month_index = (5 * day_in_cycle + 2) / 153;
    let day = u32::try_from(day_in_cycle - (153 * month_index + 2) / 5 + 1)
        .map_err(|_| CalendarError::Overflow)?;
    let month = u32::try_from(month_index + 3 - 12 * (month_index / 10))
        .map_err(|_| CalendarError::Overflow)?;
    let year = i32::try_from(100 * century + quadrennial - 4800 + month_index / 10)
        .map_err(|_| CalendarError::Overflow)?;
    NaiveDate::from_ymd_opt(year, month, day).ok_or(CalendarError::Overflow)
}

fn revised_julian_from_gregorian(date: NaiveDate) -> Result<CalendarDate, CalendarError> {
    let anchor = NaiveDate::from_ymd_opt(1923, 1, 1).expect("valid calendar anchor");
    let absolute_day = (date - anchor).num_days();
    let mut year = date.year();
    while revised_days_from_anchor_to_year_start(year + 1) <= absolute_day {
        year += 1;
    }
    while revised_days_from_anchor_to_year_start(year) > absolute_day {
        year -= 1;
    }
    let mut day_of_year = absolute_day - revised_days_from_anchor_to_year_start(year);
    let month_lengths = [
        31,
        if revised_julian_leap_year(year) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for (month, length) in (1_u32..).zip(month_lengths) {
        if day_of_year < i64::from(length) {
            return Ok(CalendarDate {
                year,
                month,
                day: u32::try_from(day_of_year + 1).map_err(|_| CalendarError::Overflow)?,
            });
        }
        day_of_year -= i64::from(length);
    }
    Err(CalendarError::Overflow)
}

fn revised_days_from_anchor_to_year_start(year: i32) -> i64 {
    const ANCHOR_YEAR: i32 = 1923;
    if year >= ANCHOR_YEAR {
        (ANCHOR_YEAR..year).map(revised_year_length).sum()
    } else {
        -(year..ANCHOR_YEAR).map(revised_year_length).sum::<i64>()
    }
}

fn revised_year_length(year: i32) -> i64 {
    if revised_julian_leap_year(year) {
        366
    } else {
        365
    }
}

fn revised_julian_leap_year(year: i32) -> bool {
    if year.rem_euclid(4) != 0 {
        return false;
    }
    if year.rem_euclid(100) != 0 {
        return true;
    }
    matches!(year.rem_euclid(900), 200 | 600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthodox_pascha_matches_published_oca_dates() {
        let cases = [
            (2023, "2023-04-16"),
            (2024, "2024-05-05"),
            (2025, "2025-04-20"),
            (2026, "2026-04-12"),
            (2027, "2027-05-02"),
            (2028, "2028-04-16"),
            (2029, "2029-04-08"),
            (2030, "2030-04-28"),
        ];
        for (year, expected) in cases {
            assert_eq!(orthodox_pascha(year).unwrap().to_string(), expected);
        }
    }

    #[test]
    fn julian_projection_tracks_old_calendar_fixed_dates() {
        let current = project_fixed_date(
            NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(),
            FixedCalendar::Julian,
        )
        .unwrap();
        assert_eq!(current.to_string(), "2025-12-25");

        let after_2100 = project_fixed_date(
            NaiveDate::from_ymd_opt(2101, 1, 8).unwrap(),
            FixedCalendar::Julian,
        )
        .unwrap();
        assert_eq!(after_2100.to_string(), "2100-12-25");
    }

    #[test]
    fn revised_julian_projection_applies_the_milankovic_leap_rule() {
        let lower_boundary = NaiveDate::from_ymd_opt(1600, 1, 1).unwrap();
        assert!(
            project_fixed_date(lower_boundary, FixedCalendar::RevisedJulian).is_ok(),
            "the documented lower boundary must terminate and project"
        );
        let date = NaiveDate::from_ymd_opt(2799, 12, 31).unwrap();
        assert_eq!(
            project_fixed_date(date, FixedCalendar::RevisedJulian)
                .unwrap()
                .to_string(),
            "2799-12-31"
        );
        let divergence = NaiveDate::from_ymd_opt(2800, 2, 29).unwrap();
        assert_eq!(
            project_fixed_date(divergence, FixedCalendar::RevisedJulian)
                .unwrap()
                .to_string(),
            "2800-03-01"
        );
        let unsupported = NaiveDate::from_ymd_opt(1599, 12, 31).unwrap();
        assert_eq!(
            project_fixed_date(unsupported, FixedCalendar::RevisedJulian),
            Err(CalendarError::RevisedJulianRange)
        );
    }

    #[test]
    fn tone_cycle_matches_all_four_dated_witnesses() {
        let tones = (1..=8)
            .map(|tone| format!("tone_{tone}"))
            .collect::<Vec<_>>();
        let cases = [
            ("2023-08-27", 3),
            ("2026-07-26", 7),
            ("2026-08-02", 8),
            ("2026-09-06", 5),
        ];
        for (date, ordinal) in cases {
            let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
            assert_eq!(octoechos_tone(date, &tones).unwrap().ordinal, ordinal);
        }
    }

    #[test]
    fn pascha_and_bright_week_do_not_claim_an_octoechos_tone() {
        let tones = (1..=8)
            .map(|tone| format!("tone_{tone}"))
            .collect::<Vec<_>>();
        let date = NaiveDate::from_ymd_opt(2026, 4, 12).unwrap();
        assert_eq!(
            octoechos_tone(date, &tones),
            Err(CalendarError::ToneCycleSuspended { date })
        );
    }

    #[test]
    fn thomas_sunday_begins_with_tone_one() {
        let tones = (1..=8)
            .map(|tone| format!("tone_{tone}"))
            .collect::<Vec<_>>();
        let date = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();
        let result = octoechos_tone(date, &tones).unwrap();
        assert_eq!(result.ordinal, 1);
        assert_eq!(result.anchor, date);
    }
}
