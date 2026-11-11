use chrono::{Datelike, Days, NaiveDate};

pub struct MonthCell {
    pub date: NaiveDate,
    // false for the leading/trailing days borrowed from neighbouring months
    pub in_month: bool,
}

// The 42 cells (6 weeks, Monday first) that cover the given month.
// None if the year/month is not a real month.
pub fn month_grid(year: i32, month: u32) -> Option<Vec<MonthCell>> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let lead = first.weekday().num_days_from_monday() as u64;
    let start = first.checked_sub_days(Days::new(lead))?;

    (0..42)
        .map(|offset| {
            let date = start.checked_add_days(Days::new(offset))?;
            Some(MonthCell {
                date,
                in_month: date.month() == month,
            })
        })
        .collect()
}

// Moves a (year, month) pair by `delta` months, rolling the year over.
pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + (month as i32 - 1) + delta;
    (
        index.div_euclid(12),
        index.rem_euclid(12) as u32 + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
    }

    #[test]
    fn grid_has_six_weeks_starting_on_a_monday() {
        // November 2026 starts on a Sunday, so six leading October days.
        let cells = month_grid(2026, 11).expect("valid month");
        assert_eq!(cells.len(), 42);
        assert_eq!(cells[0].date, date(2026, 10, 26));
        assert_eq!(cells[0].date.weekday(), chrono::Weekday::Mon);
        assert!(!cells[0].in_month);
        assert_eq!(cells[6].date, date(2026, 11, 1));
        assert!(cells[6].in_month);
    }

    #[test]
    fn grid_marks_only_the_months_own_days() {
        let cells = month_grid(2026, 11).expect("valid month");
        assert_eq!(cells.iter().filter(|c| c.in_month).count(), 30);

        // leap February
        let cells = month_grid(2028, 2).expect("valid month");
        assert_eq!(cells.iter().filter(|c| c.in_month).count(), 29);
    }

    #[test]
    fn grid_has_no_leading_days_when_month_starts_on_monday() {
        // June 2026 starts on a Monday.
        let cells = month_grid(2026, 6).expect("valid month");
        assert_eq!(cells[0].date, date(2026, 6, 1));
    }

    #[test]
    fn grid_rejects_an_invalid_month() {
        assert!(month_grid(2026, 13).is_none());
        assert!(month_grid(2026, 0).is_none());
    }

    #[test]
    fn shift_month_rolls_over_the_year() {
        assert_eq!(shift_month(2026, 11, 1), (2026, 12));
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(shift_month(2026, 3, -14), (2025, 1));
    }
}
