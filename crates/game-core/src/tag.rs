//! Dog tag numbers, K-PPP-NNN. PPP is the call-up: one per calendar month, November 2026 is the
//! first. NNN is the order of signing inside that call-up, from 001. The letter stays K; a new
//! letter comes in once K runs out.

const FIRST_YEAR: i64 = 2026;
const FIRST_MONTH: i64 = 11;
const DAY_MS: f64 = 86_400_000.0;

/// Year and month (1..=12) of a UTC day count since 1970-01-01.
fn civil(days: i64) -> (i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month)
}

/// UTC day count since 1970-01-01 of the first day of a month.
fn month_start(year: i64, month: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * ((month + 9) % 12) + 2) / 5;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Call-up of a moment in ms since 1970 (UTC): 1 for November 2026, 2 for December and so on.
pub fn callup(ms: f64) -> u32 {
    let (y, m) = civil((ms / DAY_MS).floor() as i64);
    ((y - FIRST_YEAR) * 12 + m - FIRST_MONTH + 1).clamp(1, 999) as u32
}

fn format(callup: u32, order: u32) -> String {
    format!("K-{callup:03}-{order:03}")
}

/// Next number of the moment's call-up after the tags already given out.
pub fn next<'a>(ms: f64, taken: impl Iterator<Item = &'a str>) -> Result<String, String> {
    let c = callup(ms);
    let prefix = format(c, 0);
    let prefix = &prefix[..6];
    let last = taken
        .filter_map(|t| t.strip_prefix(prefix)?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    if last >= 999 {
        return Err(format!("call-up {c:03} is full, tags need a new letter"));
    }
    Ok(format(c, last + 1))
}

/// Tag a lone device gives. With no shared counter the order follows how far into the month the
/// contract is signed, so early signers still get small numbers.
pub fn local(ms: f64) -> String {
    let (y, m) = civil((ms / DAY_MS).floor() as i64);
    let start = month_start(y, m) as f64 * DAY_MS;
    let end = if m == 12 {
        month_start(y + 1, 1)
    } else {
        month_start(y, m + 1)
    } as f64
        * DAY_MS;
    let order = 1
        + ((ms - start) / (end - start) * 999.0)
            .floor()
            .clamp(0.0, 998.0) as u32;
    format(callup(ms), order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i64, month: i64, day: i64) -> f64 {
        (month_start(year, month) + day - 1) as f64 * DAY_MS
    }

    #[test]
    fn callups_count_months_from_november_2026() {
        assert_eq!(callup(at(2026, 11, 2)), 1);
        assert_eq!(callup(at(2026, 12, 31) + DAY_MS - 1.0), 2);
        assert_eq!(callup(at(2027, 1, 1)), 3);
        assert_eq!(callup(at(2028, 2, 29)), 16);
        assert_eq!(
            callup(at(2020, 5, 1)),
            1,
            "clock before the launch still gives the first"
        );
    }

    #[test]
    fn numbers_follow_signing_order() {
        let now = at(2026, 11, 5);
        assert_eq!(next(now, [].into_iter()).unwrap(), "K-001-001");
        let given = ["K-001-001", "K-001-007", "K-002-400", "K-417-093"];
        assert_eq!(next(now, given.into_iter()).unwrap(), "K-001-008");
        assert_eq!(
            next(at(2026, 12, 1), given.into_iter()).unwrap(),
            "K-002-401"
        );
        assert!(next(now, ["K-001-999"].into_iter()).is_err());
    }

    #[test]
    fn a_lone_device_counts_through_the_month() {
        assert_eq!(local(at(2026, 11, 1)), "K-001-001");
        assert_eq!(local(at(2026, 11, 16)), "K-001-500");
        assert_eq!(local(at(2026, 12, 1) - 1.0), "K-001-999");
        assert_eq!(local(at(2027, 2, 15)), "K-004-500");
    }
}
