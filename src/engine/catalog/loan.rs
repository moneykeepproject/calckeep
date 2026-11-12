//! Loan calculators

use super::*;

pub(super) fn loan_payment() -> Calc {
    Calc {
        name: "Loan Payment",
        blurb: "Payment for a personal or general loan, monthly, biweekly or weekly.",
        fields: vec![
            Field::money("amount", "Loan amount", 25_000.0),
            rate("rate", "Interest rate (APR)", 8.0, 30.0),
            Field::slider("years", "Loan term", Unit::Years, 5.0, 1.0, 30.0, 1.0),
            Field::choice(
                "freq",
                "Payment frequency",
                &["Monthly", "Biweekly", "Weekly"],
                0,
            ),
        ],
        compute: |v| {
            let per_year = [12.0, 26.0, 52.0][v.idx("freq").min(2)];
            let n = (v.get("years") * per_year).round();
            let r = v.pct("rate") / per_year;
            let p = pmt(r, n, v.get("amount"));
            let a = amortize(v.get("amount"), r, p, 0.0, n as u32 + 1, per_year as u32);
            let mut o = Output::default();
            o.primary(
                format!(
                    "{} payment",
                    ["Monthly", "Biweekly", "Weekly"][v.idx("freq").min(2)]
                ),
                money(p),
            );
            o.row("Number of payments", count(n))
                .row("Total interest", money(a.total_interest))
                .row("Total paid", money(a.total_paid));
            o.table = Some(yearly_table("Payoff by year", &a, "Year"));
            o
        },
    }
}

pub(super) fn amortization_schedule() -> Calc {
    Calc {
        name: "Amortization Schedule",
        blurb: "Full payment schedule and how extra payments shorten the loan.",
        fields: vec![
            Field::money("amount", "Loan amount", 250_000.0),
            rate("rate", "Interest rate", 6.5, 20.0),
            Field::slider("years", "Loan term", Unit::Years, 30.0, 1.0, 40.0, 1.0),
            Field::money("extra", "Extra monthly payment", 0.0).quick(&[25.0, 50.0, 100.0, 500.0]),
            Field::choice("view", "Schedule view", &["By year", "By month"], 0),
        ],
        compute: |v| {
            let amount = v.get("amount");
            let n = v.get("years") * 12.0;
            let r = v.pct("rate") / 12.0;
            let p = pmt(r, n, amount);
            let group = if v.idx("view") == 0 { 12 } else { 1 };
            let base = amortize(amount, r, p, 0.0, n as u32 + 1, group);
            let a = amortize(amount, r, p, v.get("extra"), n as u32 + 1, group);
            let mut o = Output::default();
            o.primary("Monthly payment", money(p));
            if v.get("extra") > 0.0 {
                o.row("With extra payment", money(p + v.get("extra")));
            }
            o.row("Total interest", money(a.total_interest))
                .row("Total paid", money(a.total_paid))
                .row("Paid off in", duration(a.periods));
            if v.get("extra") > 0.0 {
                o.row(
                    "Interest saved",
                    money(base.total_interest - a.total_interest),
                )
                .row(
                    "Time saved",
                    duration(base.periods.saturating_sub(a.periods)),
                );
            }
            o.table = Some(yearly_table(
                if group == 12 {
                    "Schedule by year"
                } else {
                    "Schedule by month"
                },
                &a,
                if group == 12 { "Year" } else { "Month" },
            ));
            o
        },
    }
}

pub(super) fn loan_comparison() -> Calc {
    Calc {
        name: "Loan Comparison",
        blurb: "Compare two loan offers side by side, including fees and true APR.",
        fields: vec![
            Field::money("amount", "Amount you need", 20_000.0),
            Field::percent("ra", "Offer A interest rate", 9.0, 0.0, 36.0, 0.125),
            Field::slider("ya", "Offer A term", Unit::Years, 5.0, 1.0, 30.0, 1.0),
            Field::percent("fa", "Offer A fee", 0.0, 0.0, 10.0, 0.25)
                .help("Origination fee taken out of the loan"),
            Field::percent("rb", "Offer B interest rate", 7.5, 0.0, 36.0, 0.125),
            Field::slider("yb", "Offer B term", Unit::Years, 5.0, 1.0, 30.0, 1.0),
            Field::percent("fb", "Offer B fee", 5.0, 0.0, 10.0, 0.25),
        ],
        compute: |v| {
            let need = v.get("amount");
            let offer = |r: f64, y: f64, fee: f64| {
                let principal = need / (1.0 - fee);
                let n = y * 12.0;
                let p = pmt(r / 12.0, n, principal);
                let cost = p * n - need;
                let apr = rate_for(need, p, n).map(|x| x * 12.0).unwrap_or(r);
                (p, cost, apr, principal)
            };
            let (pa, ca, aa, la) = offer(v.pct("ra"), v.get("ya"), v.pct("fa"));
            let (pb, cb, ab, lb) = offer(v.pct("rb"), v.get("yb"), v.pct("fb"));
            let mut o = Output::default();
            if (ca - cb).abs() < 0.5 {
                o.primary("Total cost", "About the same");
            } else if ca < cb {
                o.primary("Offer A costs less by", money0(cb - ca));
            } else {
                o.primary("Offer B costs less by", money0(ca - cb));
            }
            o.row("Offer A monthly payment", money(pa))
                .row("Offer A true APR", pct2(aa * 100.0))
                .row("Offer B monthly payment", money(pb))
                .row("Offer B true APR", pct2(ab * 100.0));
            o.note("Total cost = interest + fees. True APR includes fees, so it's the fairer comparison of rates.");
            o.table = Some(Table {
                title: "Side by side".into(),
                headers: vec![
                    "Offer".into(),
                    "Borrowed".into(),
                    "Payment".into(),
                    "Total cost".into(),
                ],
                rows: vec![
                    vec!["A".into(), money0(la), money(pa), money0(ca)],
                    vec!["B".into(), money0(lb), money(pb), money0(cb)],
                ],
            });
            o
        },
    }
}

pub(super) fn heloc() -> Calc {
    Calc {
        name: "HELOC Payment",
        blurb: "Payments during the draw period and after it ends, when principal is due.",
        fields: vec![
            Field::money("amount", "Amount borrowed", 50_000.0),
            Field::percent("rate", "Interest rate", 8.5, 0.0, 15.0, 0.125)
                .help("HELOC rates are usually variable"),
            Field::slider("draw", "Draw period", Unit::Years, 10.0, 1.0, 15.0, 1.0),
            Field::slider(
                "repay",
                "Repayment period",
                Unit::Years,
                20.0,
                5.0,
                30.0,
                1.0,
            ),
            Field::choice(
                "mode",
                "During the draw period pay",
                &["Interest only", "Principal + interest"],
                0,
            ),
        ],
        compute: |v| {
            let a = v.get("amount");
            let r = v.pct("rate") / 12.0;
            let nd = v.get("draw") * 12.0;
            let nr = v.get("repay") * 12.0;
            let mut o = Output::default();
            if v.idx("mode") == 0 {
                let draw_pay = a * r;
                let repay_pay = pmt(r, nr, a);
                o.primary("Payment during draw period", money(draw_pay));
                o.row("Payment during repayment", money(repay_pay))
                    .row("Payment jump at end of draw", money(repay_pay - draw_pay))
                    .row("Interest during draw", money0(draw_pay * nd))
                    .row("Interest during repayment", money0(repay_pay * nr - a))
                    .row("Total interest", money0(draw_pay * nd + repay_pay * nr - a));
                o.note("Interest-only payments don't reduce the balance, so the payment jumps when repayment starts.");
            } else {
                let p = pmt(r, nd + nr, a);
                o.primary("Monthly payment", money(p));
                o.row("Balance at end of draw", money0(balance_after(a, r, p, nd)))
                    .row("Total interest", money0(p * (nd + nr) - a));
            }
            o
        },
    }
}

pub(super) fn student_loan() -> Calc {
    Calc {
        name: "Student Loan Payoff",
        blurb: "Standard vs. extended repayment, and how extra payments cut interest.",
        fields: vec![
            Field::money("balance", "Loan balance", 35_000.0),
            Field::percent("rate", "Interest rate", 6.5, 0.0, 15.0, 0.125),
            Field::choice(
                "plan",
                "Repayment plan",
                &["Standard 10 yr", "Extended 25 yr", "Custom"],
                0,
            ),
            Field::slider("years", "Custom term", Unit::Years, 15.0, 5.0, 30.0, 1.0)
                .show_if("plan", 2.0),
            Field::money("extra", "Extra each month", 0.0).quick(&[25.0, 50.0, 100.0, 200.0]),
        ],
        compute: |v| {
            let b = v.get("balance");
            let r = v.pct("rate") / 12.0;
            let years = match v.idx("plan") {
                0 => 10.0,
                1 => 25.0,
                _ => v.get("years"),
            };
            let n = years * 12.0;
            let p = pmt(r, n, b);
            let base = amortize(b, r, p, 0.0, n as u32 + 1, 12);
            let a = amortize(b, r, p, v.get("extra"), n as u32 + 1, 12);
            let mut o = Output::default();
            o.primary("Monthly payment", money(p + v.get("extra")));
            o.row("Paid off in", duration(a.periods))
                .row("Total interest", money0(a.total_interest));
            if v.get("extra") > 0.0 {
                o.row(
                    "Interest saved by extra",
                    money0(base.total_interest - a.total_interest),
                )
                .row(
                    "Time saved",
                    duration(base.periods.saturating_sub(a.periods)),
                );
            }
            let std = pmt(r, 120.0, b);
            let ext = pmt(r, 300.0, b);
            o.row(
                "Standard (10 yr) payment / interest",
                format!("{} / {}", money0(std), money0(std * 120.0 - b)),
            )
            .row(
                "Extended (25 yr) payment / interest",
                format!("{} / {}", money0(ext), money0(ext * 300.0 - b)),
            );
            o.note("Income-driven plans depend on income and family size and aren't modeled here.");
            o.table = Some(yearly_table("Payoff by year", &a, "Year"));
            o
        },
    }
}
