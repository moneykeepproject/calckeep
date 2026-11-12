//! Education calculators

use super::*;

pub(super) fn college_529() -> Calc {
    Calc {
        name: "529 College Savings",
        blurb: "Project a 529 plan balance at college time and the monthly savings to hit a goal.",
        fields: vec![
            Field::slider("age", "Child's age now", Unit::Years, 5.0, 0.0, 17.0, 1.0),
            Field::slider(
                "start",
                "Age starting college",
                Unit::Years,
                18.0,
                16.0,
                20.0,
                1.0,
            ),
            Field::money("saved", "Current 529 balance", 10_000.0),
            Field::money("monthly", "Monthly contribution", 250.0),
            Field::percent("ret", "Annual return", 6.0, 0.0, 12.0, 0.25),
            Field::money("goal", "Savings goal", 100_000.0)
                .help("Try the College Cost calculator for an estimate"),
        ],
        compute: |v| {
            let years = (v.get("start") - v.get("age")).max(0.0);
            let n = (years * 12.0).round();
            if n <= 0.0 {
                return Output::warning_only(
                    "College start age must be after the child's current age.",
                );
            }
            let r = v.pct("ret") / 12.0;
            let g = (1.0 + r).powf(n);
            let fv_saved = v.get("saved") * g;
            let fv_contrib = if r == 0.0 {
                v.get("monthly") * n
            } else {
                v.get("monthly") * (g - 1.0) / r
            };
            let total = fv_saved + fv_contrib;
            let gap = v.get("goal") - total;
            let needed = if gap <= 0.0 {
                0.0
            } else if r == 0.0 {
                (v.get("goal") - fv_saved) / n
            } else {
                (v.get("goal") - fv_saved).max(0.0) * r / (g - 1.0)
            };
            let mut o = Output::default();
            o.primary("Balance when college starts", money0(total));
            o.row(
                "Total contributions",
                money0(v.get("saved") + v.get("monthly") * n),
            )
            .row(
                "Investment growth",
                money0(total - v.get("saved") - v.get("monthly") * n),
            );
            if gap > 0.0 {
                o.row("Short of goal by", money0(gap))
                    .row("Monthly needed to reach goal", money(needed));
            } else {
                o.row("Above goal by", money0(-gap));
            }
            let mut rows = vec![];
            let mut b = v.get("saved");
            for y in 1..=years.round() as u32 {
                for _ in 0..12 {
                    b = b * (1.0 + r) + v.get("monthly");
                }
                rows.push(vec![format!("{}", v.get("age") as u32 + y), money0(b)]);
            }
            o.table = Some(Table {
                title: "Balance by child's age".into(),
                headers: vec!["Age".into(), "Balance".into()],
                rows,
            });
            o
        },
    }
}

pub(super) fn college_cost() -> Calc {
    Calc {
        name: "College Cost",
        blurb: "Estimate the future cost of college with tuition inflation and expected aid.",
        fields: vec![
            Field::money("cost", "Yearly cost today", 30_000.0)
                .help("Tuition, fees, room and board"),
            Field::slider(
                "until",
                "Years until college",
                Unit::Years,
                10.0,
                0.0,
                18.0,
                1.0,
            ),
            Field::slider(
                "length",
                "Years in college",
                Unit::Years,
                4.0,
                1.0,
                6.0,
                1.0,
            ),
            Field::percent("infl", "College cost inflation", 4.0, 0.0, 10.0, 0.25),
            Field::money("aid", "Grants & scholarships per year (today's $)", 5_000.0),
        ],
        compute: |v| {
            let i = v.pct("infl");
            let mut rows = vec![];
            let (mut total, mut net_total) = (0.0, 0.0);
            let start = v.get("until").round() as i32;
            for y in 0..v.get("length").round() as i32 {
                let f = (1.0 + i).powi(start + y);
                let c = v.get("cost") * f;
                let a = (v.get("aid") * f).min(c);
                total += c;
                net_total += c - a;
                rows.push(vec![
                    format!("Year {}", y + 1),
                    money0(c),
                    money0(a),
                    money0(c - a),
                ]);
            }
            let mut o = Output::default();
            o.primary("Total cost after aid", money0(net_total));
            o.row("Total cost before aid", money0(total))
                .row(
                    "First-year cost",
                    money0(v.get("cost") * (1.0 + i).powi(start)),
                )
                .row(
                    "Same total in today's dollars",
                    money0(v.get("cost") * v.get("length")),
                );
            o.table = Some(Table {
                title: "Cost by year of college".into(),
                headers: vec!["".into(), "Cost".into(), "Aid".into(), "You pay".into()],
                rows,
            });
            o
        },
    }
}
