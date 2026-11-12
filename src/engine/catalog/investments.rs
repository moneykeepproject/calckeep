//! Investments calculators

use super::*;

pub(super) fn compound_growth() -> Calc {
    Calc {
        name: "Compound Growth",
        blurb: "How an investment grows with regular contributions and compounding.",
        fields: vec![
            Field::money("initial", "Initial investment", 10_000.0),
            Field::money("monthly", "Monthly contribution", 300.0),
            Field::percent("ret", "Annual return", 7.0, 0.0, 15.0, 0.25),
            Field::slider("years", "Years to grow", Unit::Years, 20.0, 1.0, 50.0, 1.0),
            Field::percent("infl", "Inflation", 2.5, 0.0, 6.0, 0.1),
            Field::choice(
                "timing",
                "Contributions made",
                &["End of month", "Start of month"],
                0,
            ),
        ],
        compute: |v| {
            let r = v.pct("ret") / 12.0;
            let years = v.get("years").round() as u32;
            let start = v.idx("timing") == 1;
            let mut bal = v.get("initial");
            let mut contributed = bal;
            let mut rows = vec![];
            for y in 1..=years {
                for _ in 0..12 {
                    if start {
                        bal = (bal + v.get("monthly")) * (1.0 + r);
                    } else {
                        bal = bal * (1.0 + r) + v.get("monthly");
                    }
                    contributed += v.get("monthly");
                }
                rows.push(vec![
                    y.to_string(),
                    money0(contributed),
                    money0(bal - contributed),
                    money0(bal),
                ]);
            }
            let mut o = Output::default();
            o.primary("Future value", money0(bal));
            o.row("Total contributions", money0(contributed))
                .row("Total growth", money0(bal - contributed))
                .row(
                    "In today's dollars",
                    money0(bal / (1.0 + v.pct("infl")).powi(years as i32)),
                );
            o.table = Some(Table {
                title: "Growth by year".into(),
                headers: vec![
                    "Year".into(),
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

pub(super) fn roi() -> Calc {
    Calc {
        name: "Return on Investment",
        blurb: "Total and annualized return (CAGR) for an investment you bought and sold.",
        fields: vec![
            Field::money("invested", "Amount invested", 10_000.0),
            Field::money("final", "Ending value", 15_000.0),
            Field::number("years", "Years held", Unit::Years, 5.0)
                .max(100.0)
                .help("Decimals OK, e.g. 2.5"),
        ],
        compute: |v| {
            let inv = v.get("invested");
            let fin = v.get("final");
            if inv <= 0.0 {
                return Output::warning_only("Enter the amount invested.");
            }
            let gain = fin - inv;
            let mut o = Output::default();
            o.primary("Return on investment", pct2(gain / inv * 100.0));
            o.row("Total gain", money(gain))
                .row("Growth multiple", format!("{}×", trim(fin / inv, 2)));
            if v.get("years") > 0.0 && fin > 0.0 {
                o.row(
                    "Annualized return (CAGR)",
                    pct2(((fin / inv).powf(1.0 / v.get("years")) - 1.0) * 100.0),
                );
            }
            o
        },
    }
}

pub(super) fn inflation() -> Calc {
    Calc {
        name: "Inflation",
        blurb: "What things will cost in the future, or what today's money will be worth.",
        fields: vec![
            Field::choice("mode", "Calculate", &["Future cost", "Buying power"], 0),
            Field::money("amount", "Amount today", 100.0),
            Field::percent("rate", "Inflation rate", 3.0, 0.0, 15.0, 0.1),
            Field::slider("years", "Years", Unit::Years, 10.0, 1.0, 50.0, 1.0),
        ],
        compute: |v| {
            let a = v.get("amount");
            let i = v.pct("rate");
            let n = v.get("years").round() as i32;
            let future = v.idx("mode") == 0;
            let at = |y: i32| {
                if future {
                    a * (1.0 + i).powi(y)
                } else {
                    a / (1.0 + i).powi(y)
                }
            };
            let mut o = Output::default();
            if future {
                o.primary(format!("Cost in {n} years"), money(at(n)));
                o.row("Increase", money(at(n) - a)).row(
                    "Percent increase",
                    pct2((at(n) / a.max(1e-9) - 1.0) * 100.0),
                );
            } else {
                o.primary(format!("Buying power in {n} years"), money(at(n)));
                o.row("Value lost", money(a - at(n)))
                    .row("Percent lost", pct2((1.0 - at(n) / a.max(1e-9)) * 100.0));
            }
            o.table = Some(Table {
                title: "By year".into(),
                headers: vec![
                    "Year".into(),
                    if future {
                        "Cost".into()
                    } else {
                        "Buying power".into()
                    },
                ],
                rows: (1..=n).map(|y| vec![y.to_string(), money(at(y))]).collect(),
            });
            o
        },
    }
}

pub(super) fn dividend_reinvestment() -> Calc {
    Calc {
        name: "Dividend Reinvestment",
        blurb: "Growth of a dividend stock or fund with dividends reinvested or taken as cash.",
        fields: vec![
            Field::money("initial", "Initial investment", 10_000.0),
            Field::money("monthly", "Monthly addition", 0.0),
            Field::percent("yield", "Dividend yield", 3.0, 0.0, 12.0, 0.1),
            Field::percent("dgrow", "Yearly dividend growth", 5.0, 0.0, 15.0, 0.25),
            Field::percent("pgrow", "Yearly share price growth", 5.0, -5.0, 15.0, 0.25),
            Field::slider("years", "Years", Unit::Years, 20.0, 1.0, 40.0, 1.0),
            Field::choice("reinvest", "Dividends", &["Reinvest", "Take as cash"], 0),
        ],
        compute: |v| {
            let mut price = 100.0;
            let mut dps = price * v.pct("yield"); // yearly dividend per share
            let mut shares = v.get("initial") / price;
            let reinvest = v.idx("reinvest") == 0;
            let (mut cash, mut total_div, mut added) = (0.0, 0.0, v.get("initial"));
            let mut rows = vec![];
            let qg = (1.0 + v.pct("pgrow")).powf(0.25);
            let mut last_year_div = 0.0;
            for y in 1..=v.get("years").round() as u32 {
                let mut year_div = 0.0;
                for _ in 0..4 {
                    for _ in 0..3 {
                        shares += v.get("monthly") / price;
                        added += v.get("monthly");
                    }
                    price *= qg;
                    let d = shares * dps / 4.0;
                    year_div += d;
                    if reinvest {
                        shares += d / price;
                    } else {
                        cash += d;
                    }
                }
                total_div += year_div;
                last_year_div = year_div;
                dps *= 1.0 + v.pct("dgrow");
                rows.push(vec![
                    y.to_string(),
                    money0(year_div),
                    money0(shares * price),
                    money0(shares * price + cash),
                ]);
            }
            let value = shares * price;
            let mut o = Output::default();
            o.primary("Ending value", money0(value + cash));
            o.row("Total dividends", money0(total_div))
                .row("Dividend income in final year", money0(last_year_div))
                .row("Total invested", money0(added));
            if !reinvest {
                o.row("Cash dividends collected", money0(cash));
            }
            o.note("Before taxes. Dividends paid quarterly.");
            o.table = Some(Table {
                title: "By year".into(),
                headers: vec![
                    "Year".into(),
                    "Dividends".into(),
                    "Holdings".into(),
                    "Total".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn bond_yield() -> Calc {
    Calc {
        name: "Bond Yield to Maturity",
        blurb: "The annual return of a bond held to maturity at the price you pay.",
        fields: vec![
            Field::money("face", "Face (par) value", 1_000.0),
            Field::money("price", "Price you pay", 950.0),
            Field::percent("coupon", "Coupon rate", 5.0, 0.0, 15.0, 0.125),
            Field::slider(
                "years",
                "Years to maturity",
                Unit::Years,
                10.0,
                0.5,
                30.0,
                0.5,
            ),
            Field::choice(
                "freq",
                "Coupon payments",
                &["Twice a year", "Once a year"],
                0,
            ),
        ],
        compute: |v| {
            let f = if v.idx("freq") == 0 { 2.0 } else { 1.0 };
            let n = (v.get("years") * f).round().max(1.0);
            let face = v.get("face");
            let c = face * v.pct("coupon") / f;
            let price_at = |y: f64| pv_of_payments(y / f, n, c) + face / (1.0 + y / f).powf(n);
            let price = v.get("price");
            let mut o = Output::default();
            if price <= 0.0 || face <= 0.0 {
                return Output::warning_only("Enter the bond's price and face value.");
            }
            match solve(|y| price_at(y) - price, -0.5, 2.0) {
                Some(y) => {
                    o.primary("Yield to maturity", pct2(y * 100.0));
                    o.row("Current yield", pct2(c * f / price * 100.0))
                        .row(
                            if price < face {
                                "Discount to par"
                            } else {
                                "Premium over par"
                            },
                            money((face - price).abs()),
                        )
                        .row("Coupon income per year", money(c * f))
                        .row("Total coupons to maturity", money(c * n))
                        .row("Total return if held", money(c * n + face - price));
                }
                None => {
                    o.warn(
                        "Couldn't find a yield for these numbers; check the price and face value.",
                    );
                }
            }
            o
        },
    }
}
