//! Payoff calculators

use super::*;

pub(super) fn credit_card_payoff() -> Calc {
    Calc {
        name: "Credit Card Payoff",
        blurb: "How long it takes to pay off a card and what it costs in interest.",
        fields: vec![
            Field::money("balance", "Card balance", 6_000.0),
            Field::percent("apr", "Interest rate (APR)", 23.0, 0.0, 36.0, 0.25),
            Field::money("payment", "Monthly payment", 200.0),
        ],
        compute: |v| {
            let bal = v.get("balance");
            let r = v.pct("apr") / 12.0;
            let p = v.get("payment");
            let mut o = Output::default();
            let a = amortize(bal, r, p, 0.0, 1200, 12);
            if !a.paid_off {
                o.warn(format!(
                    "A {} payment never pays this off; interest alone is {} a month.",
                    money(p),
                    money(bal * r)
                ));
            } else {
                o.primary("Debt-free in", duration(a.periods));
                o.row("Total interest", money(a.total_interest))
                    .row("Total paid", money(a.total_paid));
            }
            for months in [12.0, 24.0, 36.0, 60.0] {
                o.row(
                    format!("Payment to be debt-free in {} mo", months),
                    money(pmt(r, months, bal)),
                );
            }
            if a.paid_off {
                let mut t = yearly_table("Balance by year", &a, "Year");
                t.headers[1] = "Paid down".into();
                o.table = Some(t);
            }
            o
        },
    }
}

pub(super) fn debt_snowball() -> Calc {
    let mut fields = vec![];
    for (i, (b, r, m)) in [
        (5_000.0, 22.0, 150.0),
        (3_000.0, 18.0, 90.0),
        (12_000.0, 7.0, 250.0),
    ]
    .into_iter()
    .enumerate()
    {
        let (kb, kr, km, lb, lr, lm) = [
            (
                "b1",
                "r1",
                "m1",
                "Debt 1 balance",
                "Debt 1 rate",
                "Debt 1 minimum payment",
            ),
            (
                "b2",
                "r2",
                "m2",
                "Debt 2 balance",
                "Debt 2 rate",
                "Debt 2 minimum payment",
            ),
            (
                "b3",
                "r3",
                "m3",
                "Debt 3 balance",
                "Debt 3 rate",
                "Debt 3 minimum payment",
            ),
        ][i];
        fields.push(Field::money(kb, lb, b));
        fields.push(Field::percent(kr, lr, r, 0.0, 36.0, 0.25));
        fields.push(Field::money(km, lm, m));
    }
    fields
        .push(Field::money("extra", "Extra monthly payment", 200.0).help("On top of all minimums"));
    fields.push(
        Field::choice("method", "Strategy", &["Avalanche", "Snowball"], 0)
            .help("Avalanche: highest rate first · Snowball: smallest balance first"),
    );
    Calc {
        name: "Debt Snowball / Avalanche",
        blurb: "Pay off up to three debts with a rollover plan and see the interest saved.",
        fields,
        compute: |v| {
            let debts: Vec<(f64, f64, f64)> = (1..=3)
                .map(|i| {
                    (
                        v.get(&format!("b{i}")),
                        v.pct(&format!("r{i}")) / 12.0,
                        v.get(&format!("m{i}")),
                    )
                })
                .collect();
            let avalanche = v.idx("method") == 0;
            let budget: f64 =
                debts.iter().filter(|d| d.0 > 0.0).map(|d| d.2).sum::<f64>() + v.get("extra");
            let mut bal: Vec<f64> = debts.iter().map(|d| d.0).collect();
            let mut interest = [0.0; 3];
            let mut done: Vec<Option<u32>> = bal
                .iter()
                .map(|b| if *b <= 0.0 { Some(0) } else { None })
                .collect();
            let mut month = 0;
            while done.iter().any(|d| d.is_none()) && month < 600 {
                month += 1;
                let mut left = budget;
                for i in 0..3 {
                    if bal[i] > 0.0 {
                        let int = bal[i] * debts[i].1;
                        bal[i] += int;
                        interest[i] += int;
                        let pay = debts[i].2.min(bal[i]).min(left);
                        bal[i] -= pay;
                        left -= pay;
                    }
                }
                let mut order: Vec<usize> = (0..3).filter(|&i| bal[i] > 0.0).collect();
                if avalanche {
                    order.sort_by(|&a, &b| debts[b].1.partial_cmp(&debts[a].1).unwrap());
                } else {
                    order.sort_by(|&a, &b| bal[a].partial_cmp(&bal[b]).unwrap());
                }
                for i in order {
                    if left <= 0.0 {
                        break;
                    }
                    let pay = left.min(bal[i]);
                    bal[i] -= pay;
                    left -= pay;
                }
                for i in 0..3 {
                    if done[i].is_none() && bal[i] < 0.01 {
                        bal[i] = 0.0;
                        done[i] = Some(month);
                    }
                }
            }
            let mut o = Output::default();
            if done.iter().any(|d| d.is_none()) {
                return Output::warning_only("The payments don't cover the interest; raise the extra payment or the minimums.");
            }
            let total_int: f64 = interest.iter().sum();
            // Minimum-payments-only comparison
            let mut min_int = 0.0;
            let mut min_months = 0;
            let mut min_ok = true;
            for d in &debts {
                if d.0 <= 0.0 {
                    continue;
                }
                let a = amortize(d.0, d.1, d.2, 0.0, 1200, 1200);
                if !a.paid_off {
                    min_ok = false;
                }
                min_int += a.total_interest;
                min_months = min_months.max(a.periods);
            }
            o.primary("Debt-free in", duration(month));
            o.row("Total interest", money(total_int))
                .row("Monthly budget", money(budget));
            if min_ok {
                o.row(
                    "Interest saved vs. minimums only",
                    money(min_int - total_int),
                )
                .row("Time saved", duration(min_months.saturating_sub(month)));
            } else {
                o.note("With minimum payments only, at least one debt would never be paid off.");
            }
            o.table = Some(Table {
                title: "Payoff order".into(),
                headers: vec![
                    "Debt".into(),
                    "Balance".into(),
                    "Paid off".into(),
                    "Interest".into(),
                ],
                rows: {
                    let mut idx: Vec<usize> = (0..3).filter(|&i| debts[i].0 > 0.0).collect();
                    idx.sort_by_key(|&i| done[i]);
                    idx.into_iter()
                        .map(|i| {
                            vec![
                                format!("Debt {}", i + 1),
                                money0(debts[i].0),
                                format!("Month {}", done[i].unwrap_or(0)),
                                money0(interest[i]),
                            ]
                        })
                        .collect()
                },
            });
            o
        },
    }
}

const DEBT_LABELS: [(&str, &str, &str, &str, &str, &str); 6] = [
    (
        "b1",
        "r1",
        "p1",
        "Debt 1 balance",
        "Debt 1 rate",
        "Debt 1 monthly payment",
    ),
    (
        "b2",
        "r2",
        "p2",
        "Debt 2 balance",
        "Debt 2 rate",
        "Debt 2 monthly payment",
    ),
    (
        "b3",
        "r3",
        "p3",
        "Debt 3 balance",
        "Debt 3 rate",
        "Debt 3 monthly payment",
    ),
    (
        "b4",
        "r4",
        "p4",
        "Debt 4 balance",
        "Debt 4 rate",
        "Debt 4 monthly payment",
    ),
    (
        "b5",
        "r5",
        "p5",
        "Debt 5 balance",
        "Debt 5 rate",
        "Debt 5 monthly payment",
    ),
    (
        "b6",
        "r6",
        "p6",
        "Debt 6 balance",
        "Debt 6 rate",
        "Debt 6 monthly payment",
    ),
];

pub(super) fn debt_consolidation() -> Calc {
    let defaults = [
        (8_000.0, 24.0, 250.0),
        (5_000.0, 19.0, 150.0),
        (3_000.0, 27.0, 100.0),
        (2_000.0, 22.0, 75.0),
        (1_500.0, 20.0, 60.0),
        (1_000.0, 18.0, 40.0),
    ];
    let mut fields = vec![Field::choice(
        "count",
        "Number of debts",
        &["1", "2", "3", "4", "5", "6"],
        2,
    )];
    for (i, ((kb, kr, kp, lb, lr, lp), (b, r, p))) in
        DEBT_LABELS.into_iter().zip(defaults).enumerate()
    {
        let need = i as f64; // "count" choice index i means i+1 debts
        fields.push(Field::money(kb, lb, b).show_if_at_least("count", need));
        fields.push(Field::percent(kr, lr, r, 0.0, 36.0, 0.25).show_if_at_least("count", need));
        fields.push(Field::money(kp, lp, p).show_if_at_least("count", need));
    }
    fields.push(Field::percent(
        "loan_rate",
        "Consolidation loan APR",
        11.0,
        0.0,
        36.0,
        0.25,
    ));
    fields.push(Field::choice(
        "term",
        "Consolidation loan term",
        &["24 mo", "36 mo", "48 mo", "60 mo"],
        1,
    ));
    fields.push(
        Field::percent("fee", "Origination fee", 3.0, 0.0, 10.0, 0.25)
            .help("Deducted from the loan, so you borrow a bit more"),
    );
    Calc {
        name: "Debt Consolidation",
        blurb: "Compare paying your debts as-is with one consolidation loan, fees included.",
        fields,
        compute: |v| {
            let k = v.idx("count") + 1;
            let mut total = 0.0;
            let mut cur_pay = 0.0;
            let mut cur_int = 0.0;
            let mut cur_months = 0;
            let mut rows = vec![];
            for (i, (kb, kr, kp, ..)) in DEBT_LABELS.iter().enumerate().take(k) {
                let (b, r, p) = (v.get(kb), v.pct(kr) / 12.0, v.get(kp));
                if b <= 0.0 {
                    continue;
                }
                let a = amortize(b, r, p, 0.0, 1200, 1200);
                if !a.paid_off {
                    return Output::warning_only(format!(
                        "Debt {}'s payment doesn't cover its interest ({} a month).",
                        i + 1,
                        money(b * r)
                    ));
                }
                total += b;
                cur_pay += p;
                cur_int += a.total_interest;
                cur_months = cur_months.max(a.periods);
                rows.push(vec![
                    format!("Debt {}", i + 1),
                    money0(b),
                    pct(v.get(kr)),
                    duration(a.periods),
                ]);
            }
            if total <= 0.0 {
                return Output::warning_only("Enter at least one debt balance.");
            }
            let n = [24.0, 36.0, 48.0, 60.0][v.idx("term").min(3)];
            let principal = total / (1.0 - v.pct("fee"));
            let fee = principal - total;
            let p = pmt(v.pct("loan_rate") / 12.0, n, principal);
            let new_int = p * n - principal;
            let new_cost = new_int + fee;
            let mut o = Output::default();
            if new_cost <= cur_int {
                o.primary("Consolidating saves", money0(cur_int - new_cost));
            } else {
                o.primary("Consolidating costs more by", money0(new_cost - cur_int));
            }
            o.row("New monthly payment", money(p))
                .row("Current monthly payments", money(cur_pay))
                .row("Total balances", money0(total))
                .row("Loan amount (with fee)", money0(principal))
                .row("Interest + fees (consolidated)", money0(new_cost))
                .row("Interest (current debts)", money0(cur_int))
                .row(
                    "Debt-free: now vs. consolidated",
                    format!("{} vs. {}", duration(cur_months), duration(n as u32)),
                );
            o.table = Some(Table {
                title: "Your current debts".into(),
                headers: vec![
                    "Debt".into(),
                    "Balance".into(),
                    "Rate".into(),
                    "Paid off in".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn minimum_payment() -> Calc {
    Calc {
        name: "Minimum Payment Trap",
        blurb: "How long paying only the card minimum takes, and what it really costs.",
        fields: vec![
            Field::money("balance", "Card balance", 5_000.0),
            Field::percent("apr", "Interest rate (APR)", 22.0, 0.0, 36.0, 0.25),
            Field::choice(
                "method",
                "Minimum payment rule",
                &["Interest + 1%", "% of balance"],
                0,
            )
            .help("Check your statement for your card's rule"),
            Field::percent("pct", "Minimum percent of balance", 2.0, 1.0, 5.0, 0.25)
                .show_if("method", 1.0),
            Field::money("floor", "Lowest minimum payment", 25.0),
        ],
        compute: |v| {
            let r = v.pct("apr") / 12.0;
            let mut bal = v.get("balance");
            let (mut month, mut interest, mut first) = (0u32, 0.0, 0.0);
            let mut rows = vec![];
            let (mut yp, mut yi) = (0.0, 0.0);
            while bal > 0.005 && month < 1200 {
                month += 1;
                let int = bal * r;
                let min = if v.idx("method") == 0 {
                    int + bal * 0.01
                } else {
                    bal * v.pct("pct")
                };
                let pay = min.max(v.get("floor")).min(bal + int);
                if pay <= int {
                    return Output::warning_only("This minimum never pays the card off.");
                }
                if month == 1 {
                    first = pay;
                }
                bal -= pay - int;
                interest += int;
                yp += pay - int;
                yi += int;
                if month % 12 == 0 || bal <= 0.005 {
                    rows.push(vec![
                        ((month - 1) / 12 + 1).to_string(),
                        money0(yp),
                        money0(yi),
                        money0(bal.max(0.0)),
                    ]);
                    yp = 0.0;
                    yi = 0.0;
                }
            }
            let fixed = amortize(v.get("balance"), r, first, 0.0, 1200, 1200);
            let mut o = Output::default();
            o.primary("Paying minimums takes", duration(month));
            o.row("Total interest", money0(interest))
                .row("Total paid", money0(v.get("balance") + interest))
                .row("First minimum payment", money(first));
            if fixed.paid_off {
                o.row(
                    format!("If you keep paying {} every month", money0(first)),
                    duration(fixed.periods),
                )
                .row(
                    "Interest saved that way",
                    money0(interest - fixed.total_interest),
                );
            }
            o.table = Some(Table {
                title: "Balance by year".into(),
                headers: vec![
                    "Year".into(),
                    "Paid down".into(),
                    "Interest".into(),
                    "Balance".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn payoff_vs_invest() -> Calc {
    Calc {
        name: "Pay Off Debt vs. Invest",
        blurb: "Should extra cash go to your debt or into investments? Compare net worth.",
        fields: vec![
            Field::money("balance", "Debt balance", 15_000.0),
            Field::percent("rate", "Debt interest rate", 7.0, 0.0, 30.0, 0.25),
            Field::money("payment", "Regular monthly payment", 350.0),
            Field::money("extra", "Extra cash each month", 300.0),
            Field::percent("ret", "Investment return", 7.0, 0.0, 15.0, 0.25),
            Field::percent("gtax", "Tax on investment gains", 15.0, 0.0, 40.0, 1.0),
            Field::slider("years", "Compare after", Unit::Years, 10.0, 1.0, 30.0, 1.0),
        ],
        compute: |v| {
            let r = v.pct("rate") / 12.0;
            let g = v.pct("ret") / 12.0;
            let months = (v.get("years") * 12.0) as u32;
            let sim = |extra_to_debt: bool| {
                let (mut debt, mut inv, mut contrib, mut int, mut free) =
                    (v.get("balance"), 0.0, 0.0, 0.0, None);
                for m in 1..=months {
                    inv *= 1.0 + g;
                    let mut cash = v.get("payment") + v.get("extra");
                    if debt > 0.0 {
                        let i = debt * r;
                        int += i;
                        let to_debt = if extra_to_debt {
                            cash
                        } else {
                            v.get("payment")
                        }
                        .min(debt + i);
                        debt = debt + i - to_debt;
                        cash -= to_debt;
                        if debt <= 0.005 {
                            debt = 0.0;
                            free = Some(m);
                        }
                    }
                    inv += cash;
                    contrib += cash;
                }
                let after_tax = inv - (inv - contrib).max(0.0) * v.pct("gtax");
                (after_tax - debt, int, free, after_tax, debt)
            };
            let (na, ia, fa, inva, _) = sim(true);
            let (nb, ib, fb, invb, db) = sim(false);
            if v.get("payment") <= v.get("balance") * r {
                return Output::warning_only(
                    "The regular payment doesn't cover the monthly interest.",
                );
            }
            let mut o = Output::default();
            if na >= nb {
                o.primary("Paying off debt first wins by", money0(na - nb));
            } else {
                o.primary("Investing the extra wins by", money0(nb - na));
            }
            let fmt_free = |f: Option<u32>| f.map(duration).unwrap_or_else(|| "not yet".into());
            o.row("Net worth: pay off first", money0(na))
                .row("Net worth: invest extra", money0(nb))
                .row("Debt-free (pay off first)", fmt_free(fa))
                .row("Debt-free (invest extra)", fmt_free(fb))
                .row("Interest paid (pay off first)", money0(ia))
                .row("Interest paid (invest extra)", money0(ib))
                .row(
                    "Investments after tax (pay off / invest)",
                    format!("{} / {}", money0(inva), money0(invb)),
                );
            if db > 0.0 {
                o.note(format!(
                    "Investing the extra still leaves {} of debt at the end.",
                    money0(db)
                ));
            }
            o.note("Paying off debt is a guaranteed return; investment returns are not.");
            o
        },
    }
}
