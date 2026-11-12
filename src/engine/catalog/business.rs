//! Business calculators

use super::*;

pub(super) fn break_even() -> Calc {
    Calc {
        name: "Break-Even",
        blurb: "Units and revenue needed to cover fixed costs and hit a profit goal.",
        fields: vec![
            Field::money("fixed", "Fixed costs (per month)", 10_000.0),
            Field::money("price", "Price per unit", 50.0),
            Field::money("var", "Variable cost per unit", 20.0),
            Field::money("target", "Target profit (per month)", 5_000.0),
        ],
        compute: |v| {
            let cm = v.get("price") - v.get("var");
            if cm <= 0.0 {
                return Output::warning_only(
                    "Price must be higher than the variable cost per unit to ever break even.",
                );
            }
            let units = v.get("fixed") / cm;
            let target_units = (v.get("fixed") + v.get("target")) / cm;
            let mut o = Output::default();
            o.primary("Break-even units", count(units.ceil()));
            o.row("Break-even revenue", money(units.ceil() * v.get("price")))
                .row("Contribution margin / unit", money(cm))
                .row(
                    "Contribution margin ratio",
                    pct2(cm / v.get("price") * 100.0),
                )
                .row("Units for target profit", count(target_units.ceil()))
                .row(
                    "Revenue for target profit",
                    money(target_units.ceil() * v.get("price")),
                );
            o
        },
    }
}

pub(super) fn profit_margin() -> Calc {
    Calc {
        name: "Profit Margin",
        blurb: "Margin vs. markup, and the price needed for a target margin.",
        fields: vec![
            Field::money("cost", "Cost", 60.0),
            Field::money("revenue", "Selling price / revenue", 100.0),
            Field::percent("target", "Target margin", 40.0, 0.0, 90.0, 1.0),
        ],
        compute: |v| {
            let cost = v.get("cost");
            let rev = v.get("revenue");
            let profit = rev - cost;
            let mut o = Output::default();
            if rev > 0.0 {
                o.primary("Gross margin", pct2(profit / rev * 100.0));
            } else {
                o.primary("Gross margin", "—");
            }
            o.row("Gross profit", money(profit));
            if cost > 0.0 {
                o.row("Markup", pct2(profit / cost * 100.0));
            }
            let t = v.pct("target");
            o.row(
                format!("Price for {} margin", pct(v.get("target"))),
                money(cost / (1.0 - t)),
            )
            .row(
                format!("Markup for {} margin", pct(v.get("target"))),
                pct2(t / (1.0 - t) * 100.0),
            );
            o
        },
    }
}

pub(super) fn employee_cost() -> Calc {
    Calc {
        name: "True Cost of an Employee",
        blurb: "Salary plus payroll taxes, benefits and overhead: what an employee really costs.",
        fields: vec![
            Field::money("salary", "Annual salary", 60_000.0),
            Field::money(
                "health",
                "Health insurance (employer share, yearly)",
                8_000.0,
            ),
            Field::percent("match", "Retirement match", 3.0, 0.0, 10.0, 0.5),
            Field::money("other", "Other benefits (yearly)", 1_000.0),
            Field::percent("suta", "State unemployment tax rate", 2.7, 0.0, 10.0, 0.1),
            Field::money("suta_base", "State unemployment wage base", 10_000.0),
            Field::percent("wc", "Workers' comp rate", 1.0, 0.0, 10.0, 0.1),
            Field::money("overhead", "Equipment, software, space (yearly)", 3_000.0),
        ],
        compute: |v| {
            let sal = v.get("salary");
            let ss = 0.062 * sal.min(184_500.0);
            let med = 0.0145 * sal;
            let futa = 0.006 * sal.min(7_000.0);
            let suta = v.pct("suta") * sal.min(v.get("suta_base"));
            let wc = v.pct("wc") * sal;
            let taxes = ss + med + futa + suta;
            let benefits = v.get("health") + sal * v.pct("match") + v.get("other");
            let total = sal + taxes + wc + benefits + v.get("overhead");
            let mut o = Output::default();
            o.primary("Total yearly cost", money0(total));
            o.row("Cost per hour (2,080 hrs)", money(total / 2080.0))
                .row(
                    "Multiple of salary",
                    format!("{}×", trim(total / sal.max(1.0), 2)),
                )
                .row("Payroll taxes", money0(taxes))
                .row("Benefits", money0(benefits))
                .row("Workers' comp", money0(wc))
                .row("Overhead", money0(v.get("overhead")));
            o.table = Some(Table {
                title: "Payroll tax detail".into(),
                headers: vec!["Tax".into(), "Amount".into()],
                rows: vec![
                    vec!["Social Security 6.2%".into(), money0(ss)],
                    vec!["Medicare 1.45%".into(), money0(med)],
                    vec!["FUTA 0.6% of $7k".into(), money0(futa)],
                    vec!["State unemployment".into(), money0(suta)],
                ],
            });
            o
        },
    }
}

pub(super) fn payback_npv() -> Calc {
    Calc {
        name: "Payback, NPV & IRR",
        blurb: "Is an investment worth it? Payback period, net present value and internal rate of return.",
        fields: vec![
            Field::money("cost", "Upfront investment", 50_000.0),
            Field::money("flow", "Yearly cash flow (savings or profit)", 12_000.0),
            Field::percent("growth", "Yearly cash flow growth", 0.0, -10.0, 20.0, 0.5),
            Field::slider("years", "Years", Unit::Years, 7.0, 1.0, 30.0, 1.0),
            Field::percent("disc", "Discount rate", 8.0, 0.0, 20.0, 0.25).help("Your required return or cost of capital"),
            Field::money("salvage", "Resale value at the end", 5_000.0),
        ],
        compute: |v| {
            let n = v.get("years").round() as usize;
            let mut flows = vec![-v.get("cost")];
            let mut cf = v.get("flow");
            for y in 1..=n {
                flows.push(cf + if y == n { v.get("salvage") } else { 0.0 });
                cf *= 1.0 + v.pct("growth");
            }
            let npv_v = npv(v.pct("disc"), &flows);
            let irr_v = irr(&flows);
            let mut cum = 0.0;
            let mut payback = None;
            let mut rows = vec![];
            for (t, f) in flows.iter().enumerate() {
                let prev = cum;
                cum += f;
                if payback.is_none() && t > 0 && cum >= 0.0 && *f > 0.0 {
                    payback = Some(t as f64 - 1.0 + (-prev / f));
                }
                rows.push(vec![t.to_string(), money0(*f), money0(cum), money0(f / (1.0 + v.pct("disc")).powi(t as i32))]);
            }
            let mut o = Output::default();
            o.primary("Net present value", money0(npv_v));
            o.row("Payback period", payback.map(|p| format!("{} yrs", trim(p, 1))).unwrap_or_else(|| "Not within the period".into()))
                .row("Internal rate of return", irr_v.map(|r| pct2(r * 100.0)).unwrap_or_else(|| "—".into()))
                .row("Total return", money0(flows.iter().sum::<f64>()));
            o.note(if npv_v >= 0.0 {
                "Positive NPV: the investment beats your discount rate."
            } else {
                "Negative NPV: the investment earns less than your discount rate."
            });
            o.table = Some(Table { title: "Cash flows".into(), headers: vec!["Year".into(), "Cash flow".into(), "Cumulative".into(), "Discounted".into()], rows });
            o
        },
    }
}

pub(super) fn pricing() -> Calc {
    Calc {
        name: "Markup & Pricing",
        blurb: "Set a price from cost using markup or margin, with an optional discount.",
        fields: vec![
            Field::money("cost", "Cost per item", 40.0),
            Field::choice("mode", "Price by", &["Markup", "Margin"], 0),
            Field::percent("markup", "Markup", 50.0, 0.0, 300.0, 1.0).show_if("mode", 0.0),
            Field::percent("margin", "Margin", 33.0, 0.0, 95.0, 1.0).show_if("mode", 1.0),
            Field::percent("disc", "Discount / sale", 0.0, 0.0, 60.0, 1.0),
        ],
        compute: |v| {
            let cost = v.get("cost");
            let price = if v.idx("mode") == 0 {
                cost * (1.0 + v.pct("markup"))
            } else {
                cost / (1.0 - v.pct("margin"))
            };
            let sale = price * (1.0 - v.pct("disc"));
            let mut o = Output::default();
            o.primary("Selling price", money(price));
            o.row("Profit per item", money(price - cost))
                .row(
                    "Margin",
                    pct2(if price > 0.0 {
                        (price - cost) / price * 100.0
                    } else {
                        0.0
                    }),
                )
                .row(
                    "Markup",
                    pct2(if cost > 0.0 {
                        (price - cost) / cost * 100.0
                    } else {
                        0.0
                    }),
                );
            if v.get("disc") > 0.0 {
                o.row("Sale price", money(sale))
                    .row("Profit at sale price", money(sale - cost))
                    .row(
                        "Margin at sale price",
                        pct2(if sale > 0.0 {
                            (sale - cost) / sale * 100.0
                        } else {
                            0.0
                        }),
                    );
                if sale < cost {
                    o.warn("The sale price is below cost.");
                }
            }
            o
        },
    }
}
