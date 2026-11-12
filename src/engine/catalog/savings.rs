//! Savings calculators

use super::*;

pub(super) fn cd_calc() -> Calc {
    Calc {
        name: "CD Calculator",
        blurb: "Interest earned on a certificate of deposit, APY and after-tax return.",
        fields: vec![
            Field::money("deposit", "Initial deposit", 10_000.0),
            Field::percent("rate", "Interest rate", 4.0, 0.0, 8.0, 0.05),
            Field::slider("months", "Term", Unit::Months, 12.0, 1.0, 120.0, 1.0),
            Field::choice(
                "comp",
                "Compounding",
                &["Daily", "Monthly", "Quarterly", "Yearly"],
                0,
            ),
            Field::percent("taxrate", "Tax rate on interest", 22.0, 0.0, 50.0, 1.0),
        ],
        compute: |v| {
            let n = [365.0, 12.0, 4.0, 1.0][v.idx("comp").min(3)];
            let r = v.pct("rate");
            let p = v.get("deposit");
            let months = v.get("months").round().max(1.0) as u32;
            let at = |m: u32| p * (1.0 + r / n).powf(n * m as f64 / 12.0);
            let end = at(months);
            let interest = end - p;
            let apy = (1.0 + r / n).powf(n) - 1.0;
            let after_tax = interest * (1.0 - v.pct("taxrate"));
            let mut o = Output::default();
            o.primary("Ending balance", money(end));
            o.row("Interest earned", money(interest))
                .row("APY", pct2(apy * 100.0))
                .row("Interest after tax", money(after_tax))
                .row("Ending balance after tax", money(p + after_tax));
            let step = if months <= 24 { 1 } else { 12 };
            let mut rows = vec![];
            let mut m = step;
            let mut prev = p;
            loop {
                let m_cap = m.min(months);
                let b = at(m_cap);
                rows.push(vec![
                    m_cap.to_string(),
                    money(b - prev),
                    money(b - p),
                    money(b),
                ]);
                prev = b;
                if m_cap == months {
                    break;
                }
                m += step;
            }
            o.table = Some(Table {
                title: "Growth over the term".into(),
                headers: vec![
                    "Month".into(),
                    "Interest".into(),
                    "Total int.".into(),
                    "Balance".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn savings_goal() -> Calc {
    Calc {
        name: "Savings Goal",
        blurb: "How much to save each month to reach a goal by a certain time.",
        fields: vec![
            Field::money("goal", "Savings goal", 20_000.0),
            Field::money("current", "Already saved", 2_000.0),
            Field::slider(
                "months",
                "Time to reach it",
                Unit::Months,
                36.0,
                1.0,
                240.0,
                1.0,
            ),
            Field::percent("rate", "Interest rate (APY)", 4.0, 0.0, 10.0, 0.05),
        ],
        compute: |v| {
            let n = v.get("months").round();
            let r = (1.0 + v.pct("rate")).powf(1.0 / 12.0) - 1.0;
            let grown = v.get("current") * (1.0 + r).powf(n);
            let need = v.get("goal") - grown;
            let mut o = Output::default();
            if need <= 0.0 {
                o.primary("Monthly savings needed", "$0");
                o.note("What you've already saved reaches the goal with interest alone.");
                return o;
            }
            let monthly = if r == 0.0 {
                need / n
            } else {
                need * r / ((1.0 + r).powf(n) - 1.0)
            };
            o.primary("Monthly savings needed", money(monthly));
            o.row("Total you'll deposit", money0(monthly * n))
                .row(
                    "Interest earned",
                    money0(v.get("goal") - v.get("current") - monthly * n),
                )
                .row("Weekly equivalent", money(monthly * 12.0 / 52.0));
            o
        },
    }
}

pub(super) fn emergency_fund() -> Calc {
    Calc {
        name: "Emergency Fund",
        blurb: "How big your emergency fund should be and how long it will take to build.",
        fields: vec![
            Field::money("expenses", "Monthly essential expenses", 4_000.0)
                .help("Housing, food, utilities, insurance, minimum debt payments"),
            Field::slider(
                "months",
                "Months of coverage",
                Unit::Months,
                6.0,
                3.0,
                12.0,
                1.0,
            ),
            Field::money("current", "Already saved", 5_000.0),
            Field::money("saving", "You can save per month", 500.0),
            Field::percent("rate", "Savings interest rate (APY)", 4.0, 0.0, 10.0, 0.05),
        ],
        compute: |v| {
            let target = v.get("expenses") * v.get("months");
            let gap = target - v.get("current");
            let mut o = Output::default();
            o.primary("Emergency fund target", money0(target));
            if gap <= 0.0 {
                o.row("Fully funded", "Yes")
                    .row("Extra above target", money0(-gap));
                return o;
            }
            let r = (1.0 + v.pct("rate")).powf(1.0 / 12.0) - 1.0;
            let (mut b, mut m) = (v.get("current"), 0u32);
            while b < target && m < 1200 && v.get("saving") > 0.0 {
                b = b * (1.0 + r) + v.get("saving");
                m += 1;
            }
            o.row("Still needed", money0(gap)).row(
                "Currently covers",
                format!(
                    "{} months",
                    trim(v.get("current") / v.get("expenses").max(1.0), 1)
                ),
            );
            if v.get("saving") > 0.0 {
                o.row("Time to fully fund", duration(m));
            }
            o.note("3 months is a common minimum; 6–12 months if your income is irregular or you're the only earner.");
            o
        },
    }
}

pub(super) fn cd_ladder() -> Calc {
    let mut fields = vec![
        Field::money("total", "Total to invest", 25_000.0),
        Field::choice("rungs", "Number of CDs", &["2", "3", "4", "5"], 3)
            .help("Rung 1 is a 1-year CD, rung 2 a 2-year CD, and so on"),
    ];
    let rates = [4.0, 3.9, 3.8, 3.75, 3.7];
    let keys = ["r1", "r2", "r3", "r4", "r5"];
    let labels = [
        "1-year CD APY",
        "2-year CD APY",
        "3-year CD APY",
        "4-year CD APY",
        "5-year CD APY",
    ];
    for i in 0..5 {
        let mut f = Field::percent(keys[i], labels[i], rates[i], 0.0, 8.0, 0.05);
        if i >= 2 {
            f = f.show_if_at_least("rungs", (i - 1) as f64); // choice index 0 = 2 CDs
        }
        fields.push(f);
    }
    Calc {
        name: "CD Ladder",
        blurb:
            "Split money across CDs that mature a year apart for steady access and better rates.",
        fields,
        compute: |v| {
            let k = v.idx("rungs") + 2;
            let each = v.get("total") / k as f64;
            let mut rows = vec![];
            let mut interest = 0.0;
            let mut first_year = 0.0;
            for i in 0..k {
                let apy = v.pct(["r1", "r2", "r3", "r4", "r5"][i]);
                let years = (i + 1) as i32;
                let end = each * (1.0 + apy).powi(years);
                interest += end - each;
                first_year += each * apy;
                rows.push(vec![
                    format!("{} yr", years),
                    pct(apy * 100.0),
                    money0(each),
                    money0(end),
                ]);
            }
            let mut o = Output::default();
            o.primary("Interest in the first year", money0(first_year));
            o.row(
                "Blended APY",
                pct2(first_year / v.get("total").max(1.0) * 100.0),
            )
            .row("Amount per CD", money0(each))
            .row("Interest by each CD's maturity", money0(interest))
            .row("A CD matures every", "12 months");
            o.note(format!(
                "When each CD matures, reinvest it in a new {}-year CD to keep the ladder going.",
                k
            ));
            o.table = Some(Table {
                title: "Ladder".into(),
                headers: vec![
                    "Term".into(),
                    "APY".into(),
                    "Deposit".into(),
                    "At maturity".into(),
                ],
                rows,
            });
            o
        },
    }
}
