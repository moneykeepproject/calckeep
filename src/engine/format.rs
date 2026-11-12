//! Number formatting helpers (no locale crates, works on WASM and mobile).

use super::{Field, Input, Unit};

/// Insert thousands separators into the integer part of a plain number string.
pub fn group(s: &str) -> String {
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
        ("-", r)
    } else {
        ("", s)
    };
    let (int, frac) = match rest.find('.') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut out = String::new();
    let len = int.len();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}{frac}")
}

fn fixed(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v);
    if s == "-0" || s.starts_with("-0.") && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_string()
    } else {
        s
    }
}

/// "$1,234.56"
pub fn money(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let s = group(&fixed(v.abs(), 2));
    if v < -0.005 {
        format!("-${s}")
    } else {
        format!("${s}")
    }
}

/// "$1,235" (whole dollars, for tables and large totals)
pub fn money0(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let s = group(&fixed(v.abs(), 0));
    if v <= -0.5 {
        format!("-${s}")
    } else {
        format!("${s}")
    }
}

/// Trim trailing zeros: 6.500 -> 6.5, 7.000 -> 7
pub fn trim(v: f64, max_decimals: usize) -> String {
    let s = fixed(v, max_decimals);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// "6.875%"
pub fn pct(v: f64) -> String {
    format!("{}%", trim(v, 3))
}

/// Percent with 2 fixed decimals: "12.50%"
pub fn pct2(v: f64) -> String {
    format!("{}%", fixed(v, 2))
}

pub fn count(v: f64) -> String {
    group(&fixed(v, 0))
}

/// "4 yrs 7 mo" from a number of months
pub fn duration(months: u32) -> String {
    let y = months / 12;
    let m = months % 12;
    match (y, m) {
        (0, m) => format!("{m} mo"),
        (y, 0) => format!("{y} yr{}", if y == 1 { "" } else { "s" }),
        (y, m) => format!("{y} yr{} {m} mo", if y == 1 { "" } else { "s" }),
    }
}

/// How a field value is shown on the calculator page.
pub fn field_display(f: &Field, v: f64) -> String {
    match &f.input {
        Input::Choice(opts) => opts
            .get(v as usize)
            .map(|s| s.to_string())
            .unwrap_or_default(),
        _ => match f.unit {
            Unit::Money => {
                if (v - v.round()).abs() < 0.005 {
                    money0(v)
                } else {
                    money(v)
                }
            }
            Unit::Percent => pct(v),
            Unit::Years => format!(
                "{} yr{}",
                trim(v, 1),
                if (v - 1.0).abs() < 1e-9 { "" } else { "s" }
            ),
            Unit::Months => format!("{} mo", trim(v, 0)),
            Unit::Count => count(v),
            Unit::Plain => group(&trim(v, 4)),
        },
    }
}

/// Raw editable text for the keypad ("350000", "6.5").
pub fn field_raw(f: &Field, v: f64) -> String {
    trim(v, f.decimals())
}

/// Keypad prefix/suffix shown around the typed text.
pub fn affixes(f: &Field) -> (&'static str, &'static str) {
    match f.unit {
        Unit::Money => ("$", ""),
        Unit::Percent => ("", "%"),
        Unit::Years => ("", " yrs"),
        Unit::Months => ("", " mo"),
        _ => ("", ""),
    }
}

/// Label for a keypad quick-add chip ("+$1k", "+0.125%", "+5 yrs").
pub fn quick_label(f: &Field, q: f64) -> String {
    match f.unit {
        Unit::Money => {
            if q >= 1_000_000.0 {
                format!("+${}M", trim(q / 1_000_000.0, 1))
            } else if q >= 1_000.0 {
                format!("+${}k", trim(q / 1_000.0, 1))
            } else {
                format!("+${}", trim(q, 0))
            }
        }
        Unit::Percent => format!("+{}%", trim(q, 3)),
        Unit::Years => format!("+{} yr", trim(q, 1)),
        Unit::Months => format!("+{} mo", trim(q, 0)),
        _ => format!("+{}", trim(q, 2)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grouping() {
        assert_eq!(group("1234567.5"), "1,234,567.5");
        assert_eq!(group("123"), "123");
        assert_eq!(group("1000"), "1,000");
        assert_eq!(group("12."), "12.");
        assert_eq!(money(-1234.5), "-$1,234.50");
        assert_eq!(money0(349999.6), "$350,000");
        assert_eq!(pct(6.875), "6.875%");
        assert_eq!(pct(7.0), "7%");
        assert_eq!(duration(55), "4 yrs 7 mo");
        assert_eq!(money(-0.001), "$0.00");
    }
}
