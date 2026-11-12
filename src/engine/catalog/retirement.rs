//! Retirement calculators

use super::*;

pub(super) fn retirement_savings() -> Calc {
    Calc {
        name: "Retirement Savings",
        blurb: "Project your nest egg and the income it can support in retirement.",
        fields: vec![
            Field::slider("age", "Current age", Unit::Years, 35.0, 18.0, 75.0, 1.0),
            Field::slider(
                "retire",
                "Retirement age",
                Unit::Years,
                67.0,
                45.0,
                80.0,
                1.0,
            ),
            Field::money("savings", "Current savings", 50_000.0),
            Field::money("monthly", "Monthly contribution", 500.0),
            Field::percent("ret", "Annual return", 6.0, 0.0, 12.0, 0.25),
            Field::percent("raise", "Yearly contribution increase", 2.0, 0.0, 10.0, 0.5),
            Field::percent("infl", "Inflation", 2.5, 0.0, 6.0, 0.1),
            Field::percent("wd", "Withdrawal rate", 4.0, 2.0, 8.0, 0.25)
                .help("Share of savings withdrawn per year"),
        ],
        compute: |v| {
            let years = (v.get("retire") - v.get("age")).round() as i32;
            if years <= 0 {
                return Output::warning_only(
                    "Retirement age must be greater than your current age.",
                );
            }
            let r = v.pct("ret") / 12.0;
            let mut bal = v.get("savings");
            let mut contrib_m = v.get("monthly");
            let mut contributed = bal;
            let mut rows = vec![];
            for y in 1..=years {
                for _ in 0..12 {
                    bal = bal * (1.0 + r) + contrib_m;
                    contributed += contrib_m;
                }
                rows.push(vec![
                    format!("{}", v.get("age") as i32 + y),
                    money0(contributed),
                    money0(bal - contributed),
                    money0(bal),
                ]);
                contrib_m *= 1.0 + v.pct("raise");
            }
            let deflate = (1.0 + v.pct("infl")).powi(years);
            let income = bal * v.pct("wd");
            let mut o = Output::default();
            o.primary("Savings at retirement", money0(bal));
            o.row("In today's dollars", money0(bal / deflate))
                .row("Total contributions", money0(contributed))
                .row("Investment growth", money0(bal - contributed))
                .row("Monthly income in retirement", money(income / 12.0))
                .row(
                    "Monthly income (today's dollars)",
                    money(income / 12.0 / deflate),
                );
            o.table = Some(Table {
                title: "Growth by age".into(),
                headers: vec![
                    "Age".into(),
                    "Contributed".into(),
                    "Growth".into(),
                    "Balance".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn k401() -> Calc {
    Calc {
        name: "401(k) Growth",
        blurb: "Your 401(k) balance at retirement with employer match and IRS limits.",
        fields: vec![
            Field::money("salary", "Annual salary", 75_000.0),
            Field::percent("contrib", "Your contribution", 6.0, 0.0, 50.0, 1.0)
                .help("Percent of salary"),
            Field::percent("match", "Employer match", 50.0, 0.0, 100.0, 5.0)
                .help("Percent of your contribution matched"),
            Field::percent("limit", "Match limit", 6.0, 0.0, 10.0, 0.5)
                .help("Match applies up to this % of salary"),
            Field::money("balance", "Current 401(k) balance", 25_000.0),
            Field::slider("age", "Current age", Unit::Years, 30.0, 18.0, 70.0, 1.0),
            Field::slider(
                "retire",
                "Retirement age",
                Unit::Years,
                67.0,
                50.0,
                75.0,
                1.0,
            ),
            Field::percent("raise", "Yearly salary increase", 3.0, 0.0, 10.0, 0.5),
            Field::percent("ret", "Annual return", 6.0, 0.0, 12.0, 0.25),
        ],
        compute: |v| {
            let years = (v.get("retire") - v.get("age")).round() as i32;
            if years <= 0 {
                return Output::warning_only(
                    "Retirement age must be greater than your current age.",
                );
            }
            let r = v.pct("ret") / 12.0;
            let mut salary = v.get("salary");
            let mut bal = v.get("balance");
            let (mut mine, mut theirs) = (0.0, 0.0);
            let mut rows = vec![];
            let mut capped = false;
            for y in 0..years {
                let age = v.get("age") as i32 + y;
                // 2026 IRS limits, held flat: $24,500 + $8,000 catch-up at 50+ ($11,250 at 60-63)
                let catch_up = if (60..=63).contains(&age) {
                    11_250.0
                } else if age >= 50 {
                    8_000.0
                } else {
                    0.0
                };
                let want = salary * v.pct("contrib");
                let emp = want.min(24_500.0 + catch_up);
                capped |= emp < want;
                let m = v.pct("match") * emp.min(salary * v.pct("limit"));
                for _ in 0..12 {
                    bal = bal * (1.0 + r) + (emp + m) / 12.0;
                }
                mine += emp;
                theirs += m;
                rows.push(vec![
                    format!("{}", age + 1),
                    money0(salary),
                    money0(emp + m),
                    money0(bal),
                ]);
                salary *= 1.0 + v.pct("raise");
            }
            let mut o = Output::default();
            o.primary("401(k) at retirement", money0(bal));
            o.row("Your contributions", money0(mine))
                .row("Employer match", money0(theirs))
                .row("Starting balance", money0(v.get("balance")))
                .row(
                    "Investment growth",
                    money0(bal - mine - theirs - v.get("balance")),
                );
            if capped {
                o.note("Some years hit the IRS contribution limit.");
            }
            o.note("Uses 2026 IRS limits ($24,500, plus $8,000 catch-up at 50+ or $11,250 at ages 60–63), held flat.");
            o.table = Some(Table {
                title: "Growth by age".into(),
                headers: vec![
                    "Age".into(),
                    "Salary".into(),
                    "Added".into(),
                    "Balance".into(),
                ],
                rows,
            });
            o
        },
    }
}

/// Social Security benefit as a fraction of the full (FRA) benefit when claiming at `age`.
pub(super) fn ss_factor(age: f64, fra: f64) -> f64 {
    let months = ((age - fra) * 12.0).round();
    if months >= 0.0 {
        1.0 + months.min(((70.0 - fra) * 12.0).round()) * (2.0 / 3.0) / 100.0 // 8% a year delayed credits
    } else {
        let early = -months;
        let first = early.min(36.0);
        1.0 - first * (5.0 / 9.0) / 100.0 - (early - first) * (5.0 / 12.0) / 100.0
    }
}

pub(super) fn social_security() -> Calc {
    Calc {
        name: "Social Security Claiming Age",
        blurb: "Monthly benefit at 62, full retirement age and 70, and the break-even ages.",
        fields: vec![
            Field::money("pia", "Benefit at full retirement age", 2_200.0)
                .help("Monthly amount from your SSA statement (ssa.gov/myaccount)"),
            Field::choice("born", "Year of birth", &["1959", "1960 or later"], 1)
                .help("Full retirement age: 66 and 10 months / 67"),
            Field::slider(
                "claim",
                "Age you plan to claim",
                Unit::Years,
                67.0,
                62.0,
                70.0,
                1.0,
            ),
            Field::slider("live", "Plan to age", Unit::Years, 85.0, 70.0, 100.0, 1.0),
        ],
        compute: |v| {
            let fra = if v.idx("born") == 0 {
                66.0 + 10.0 / 12.0
            } else {
                67.0
            };
            let pia = v.get("pia");
            let live = v.get("live");
            let monthly = |age: f64| pia * ss_factor(age, fra);
            let total = |age: f64| monthly(age) * ((live - age) * 12.0).max(0.0);
            // age at which claiming later catches up with claiming earlier
            let breakeven = |early: f64, late: f64| {
                let (me, ml) = (monthly(early), monthly(late));
                if ml <= me {
                    return None;
                }
                // me*(t-early) = ml*(t-late)  ->  t = (ml*late - me*early)/(ml-me)
                Some((ml * late - me * early) / (ml - me))
            };
            let age_str = |a: f64| {
                let y = a.floor();
                let m = ((a - y) * 12.0).round() as i32;
                if m == 0 {
                    format!("{y}")
                } else {
                    format!("{y} and {m} mo")
                }
            };
            let claim = v.get("claim");
            let mut o = Output::default();
            o.primary(
                format!("Monthly benefit at {}", claim),
                money0(monthly(claim)),
            );
            o.row(
                "Percent of full benefit",
                pct2(ss_factor(claim, fra) * 100.0),
            )
            .row(
                format!("Lifetime benefits to age {}", live),
                money0(total(claim)),
            )
            .row("Full retirement age", age_str(fra));
            if let Some(b) = breakeven(62.0, fra) {
                o.row("Waiting from 62 to full age pays off at", age_str(b));
            }
            if let Some(b) = breakeven(fra, 70.0) {
                o.row("Waiting from full age to 70 pays off at", age_str(b));
            }
            o.note("Amounts are in today's dollars (yearly cost-of-living raises apply to every claiming age). Spousal and survivor benefits aren't included.");
            let mut rows = vec![];
            for a in 62..=70 {
                let a = a as f64;
                rows.push(vec![
                    format!("{a}"),
                    money0(monthly(a)),
                    format!("{}%", trim(ss_factor(a, fra) * 100.0, 1)),
                    money0(total(a)),
                ]);
            }
            o.table = Some(Table {
                title: format!("By claiming age (lifetime to {})", live),
                headers: vec![
                    "Age".into(),
                    "Monthly".into(),
                    "% of full".into(),
                    "Lifetime".into(),
                ],
                rows,
            });
            o
        },
    }
}

fn months_lasting(bal: f64, w: f64, ret: f64, infl: f64) -> (Option<u32>, Vec<Vec<String>>) {
    let mut b = bal;
    let mut w = w;
    let mut rows = vec![];
    let mut yw = 0.0;
    for m in 1..=1200u32 {
        b *= 1.0 + ret / 12.0;
        let take = w.min(b);
        b -= take;
        yw += take;
        if m % 12 == 0 || b <= 0.005 {
            if rows.len() < 60 {
                rows.push(vec![((m - 1) / 12 + 1).to_string(), money0(yw), money0(b)]);
            }
            yw = 0.0;
        }
        if b <= 0.005 {
            return (Some(m), rows);
        }
        if m % 12 == 0 {
            w *= 1.0 + infl;
        }
    }
    (None, rows)
}

pub(super) fn savings_last() -> Calc {
    Calc {
        name: "How Long Will Savings Last",
        blurb: "How many years your nest egg lasts with withdrawals that keep up with inflation.",
        fields: vec![
            Field::money("balance", "Savings balance", 500_000.0),
            Field::money("withdraw", "Monthly withdrawal", 3_000.0),
            Field::percent("ret", "Annual return", 5.0, 0.0, 12.0, 0.25),
            Field::percent("infl", "Yearly withdrawal increase", 2.5, 0.0, 6.0, 0.1)
                .help("To keep up with inflation"),
            Field::slider(
                "target",
                "Want it to last",
                Unit::Years,
                30.0,
                5.0,
                50.0,
                1.0,
            ),
        ],
        compute: |v| {
            let (bal, ret, infl) = (v.get("balance"), v.pct("ret"), v.pct("infl"));
            let (m, rows) = months_lasting(bal, v.get("withdraw"), ret, infl);
            let target = (v.get("target") * 12.0) as u32;
            let max_w = solve(
                |w| match months_lasting(bal, w, ret, infl).0 {
                    Some(mm) => mm as f64 - target as f64,
                    None => 1e6,
                },
                1.0,
                bal,
            );
            let mut o = Output::default();
            match m {
                Some(m) => o.primary("Your savings last", duration(m)),
                None => o.primary("Your savings last", "100+ years"),
            };
            if let Some(w) = max_w {
                o.row(
                    format!("Max withdrawal to last {} yrs", v.get("target")),
                    money0(w),
                );
            }
            o.row("First-year withdrawals", money0(v.get("withdraw") * 12.0));
            o.table = Some(Table {
                title: "Balance by year".into(),
                headers: vec!["Year".into(), "Withdrawn".into(), "Balance".into()],
                rows,
            });
            o
        },
    }
}

pub(super) fn roth_vs_traditional() -> Calc {
    Calc {
        name: "Roth vs. Traditional",
        blurb:
            "Which account leaves you more after taxes: pay tax now (Roth) or later (traditional)?",
        fields: vec![
            Field::money("contrib", "Yearly contribution (pre-tax)", 7_500.0)
                .help("2026 IRA limit: $7,500 (401(k): $24,500)"),
            Field::slider(
                "years",
                "Years until retirement",
                Unit::Years,
                30.0,
                1.0,
                50.0,
                1.0,
            ),
            Field::percent("ret", "Annual return", 7.0, 0.0, 12.0, 0.25),
            Field::percent("now", "Tax rate now", 22.0, 0.0, 50.0, 1.0),
            Field::percent("later", "Tax rate in retirement", 15.0, 0.0, 50.0, 1.0),
        ],
        compute: |v| {
            let r = v.pct("ret");
            let n = v.get("years").round() as i32;
            let fv = |c: f64| c * (((1.0 + r).powi(n) - 1.0) / r.max(1e-12)) * (1.0 + r);
            let fv = |c: f64| if r == 0.0 { c * n as f64 } else { fv(c) };
            let trad_gross = fv(v.get("contrib"));
            let trad = trad_gross * (1.0 - v.pct("later"));
            let roth = fv(v.get("contrib") * (1.0 - v.pct("now")));
            let mut o = Output::default();
            if (roth - trad).abs() < 1.0 {
                o.primary("After-tax result", "About the same");
            } else if roth > trad {
                o.primary("Roth leaves you more by", money0(roth - trad));
            } else {
                o.primary("Traditional leaves you more by", money0(trad - roth));
            }
            o.row("Roth after tax", money0(roth))
                .row("Traditional after tax", money0(trad))
                .row("Traditional before tax", money0(trad_gross))
                .row(
                    "Roth yearly contribution (after tax)",
                    money0(v.get("contrib") * (1.0 - v.pct("now"))),
                );
            o.note("Same take-home cost either way. Roth wins if your tax rate later is higher than now; traditional wins if it's lower.");
            o
        },
    }
}

/// IRS Uniform Lifetime Table (2022+), ages 72..=120
const ULT: [f64; 49] = [
    27.4, 26.5, 25.5, 24.6, 23.7, 22.9, 22.0, 21.1, 20.2, 19.4, 18.5, 17.7, 16.8, 16.0, 15.2, 14.4,
    13.7, 12.9, 12.2, 11.5, 10.8, 10.1, 9.5, 8.9, 8.4, 7.8, 7.3, 6.8, 6.4, 6.0, 5.6, 5.2, 4.9, 4.6,
    4.3, 4.1, 3.9, 3.7, 3.5, 3.4, 3.3, 3.1, 3.0, 2.9, 2.8, 2.7, 2.5, 2.3, 2.0,
];

pub(super) fn ult_divisor(age: u32) -> f64 {
    ULT[(age.clamp(72, 120) - 72) as usize]
}

pub(super) fn rmd() -> Calc {
    Calc {
        name: "Required Minimum Distribution",
        blurb: "This year's RMD from an IRA or 401(k), and projected RMDs in later years.",
        fields: vec![
            Field::slider(
                "age",
                "Your age this year",
                Unit::Years,
                75.0,
                73.0,
                100.0,
                1.0,
            ),
            Field::money("balance", "Balance on Dec 31 last year", 500_000.0),
            Field::percent("ret", "Expected annual return", 5.0, 0.0, 10.0, 0.25),
        ],
        compute: |v| {
            let age = v.get("age") as u32;
            let mut bal = v.get("balance");
            let first = bal / ult_divisor(age);
            let mut rows = vec![];
            let mut total = 0.0;
            for a in age..=age.max(95).min(age + 25) {
                let d = ult_divisor(a);
                let r = bal / d;
                total += r;
                bal = (bal - r) * (1.0 + v.pct("ret"));
                rows.push(vec![a.to_string(), format!("{d}"), money0(r), money0(bal)]);
            }
            let mut o = Output::default();
            o.primary("RMD this year", money(first));
            o.row(
                "Divisor (IRS Uniform Lifetime Table)",
                format!("{}", ult_divisor(age)),
            )
            .row("Percent of balance", pct2(100.0 / ult_divisor(age)))
            .row("Monthly equivalent", money(first / 12.0))
            .row("Projected RMDs in table", money0(total));
            o.note("RMDs start at 73 (75 if born 1960 or later). Uses the Uniform Lifetime Table; a spouse more than 10 years younger who is your sole beneficiary uses a different table. Roth IRAs have no RMDs for the owner.");
            o.table = Some(Table {
                title: "Projected RMDs".into(),
                headers: vec![
                    "Age".into(),
                    "Divisor".into(),
                    "RMD".into(),
                    "Year-end bal.".into(),
                ],
                rows,
            });
            o
        },
    }
}
