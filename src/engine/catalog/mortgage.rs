//! Mortgage calculators

use super::*;

pub(super) fn mortgage_payment() -> Calc {
    Calc {
        name: "Mortgage Payment",
        blurb: "Monthly payment with taxes, insurance, PMI and HOA, plus an amortization schedule.",
        fields: vec![
            Field::money("price", "Home price", 400_000.0),
            Field::percent("down", "Down payment", 20.0, 0.0, 60.0, 0.5),
            rate("rate", "Interest rate", 6.5, 12.0),
            Field::choice("term", "Loan term", &MORTGAGE_TERMS, 3),
            Field::percent("tax", "Property tax (yearly)", 1.1, 0.0, 4.0, 0.05)
                .help("Percent of home value per year"),
            Field::money("ins", "Home insurance (yearly)", 1_800.0),
            Field::money("hoa", "HOA dues (monthly)", 0.0),
            Field::percent("pmi", "PMI rate (yearly)", 0.5, 0.0, 2.0, 0.05)
                .help("Charged when down payment is under 20%"),
        ],
        compute: |v: &Values| {
            let price = v.get("price");
            let down = price * v.pct("down");
            let loan = (price - down).max(0.0);
            let n = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let r = v.pct("rate") / 12.0;
            let pi = pmt(r, n, loan);
            let tax = price * v.pct("tax") / 12.0;
            let ins = v.get("ins") / 12.0;
            let hoa = v.get("hoa");
            let pmi = if v.get("down") < 20.0 {
                loan * v.pct("pmi") / 12.0
            } else {
                0.0
            };
            let total = pi + tax + ins + pmi + hoa;
            let a = amortize(loan, r, pi, 0.0, n as u32 + 1, 12);
            let mut o = Output::default();
            o.primary("Monthly payment", money(total));
            o.row("Principal & interest", money(pi))
                .row("Property tax", money(tax))
                .row("Home insurance", money(ins));
            if pmi > 0.0 {
                o.row("PMI", money(pmi));
            }
            if hoa > 0.0 {
                o.row("HOA dues", money(hoa));
            }
            o.row("Loan amount", money(loan))
                .row("Down payment", money(down))
                .row("Total interest", money(a.total_interest))
                .row("Total of P&I payments", money(a.total_paid));
            if pmi > 0.0 {
                o.note("PMI usually drops off once you reach 20–22% equity.");
            }
            o.table = Some(yearly_table("Amortization by year", &a, "Year"));
            o
        },
    }
}

pub(super) fn mortgage_affordability() -> Calc {
    Calc {
        name: "Home Affordability",
        blurb: "How much house your income supports using lender debt-to-income ratios.",
        fields: vec![
            Field::money("income", "Annual gross income", 100_000.0),
            Field::money("debts", "Monthly debt payments", 500.0)
                .help("Car, student loans, card minimums"),
            Field::money("down", "Down payment", 60_000.0),
            rate("rate", "Interest rate", 6.5, 12.0),
            Field::choice("term", "Loan term", &MORTGAGE_TERMS, 3),
            Field::percent("tax", "Property tax (yearly)", 1.1, 0.0, 4.0, 0.05),
            Field::money("ins", "Home insurance (yearly)", 1_800.0),
            Field::percent("front", "Max housing ratio", 28.0, 20.0, 40.0, 1.0)
                .help("Housing payment ÷ gross income"),
            Field::percent("back", "Max total debt ratio", 36.0, 30.0, 50.0, 1.0)
                .help("All debts ÷ gross income"),
        ],
        compute: |v| {
            let gross_m = v.get("income") / 12.0;
            let housing_cap = gross_m * v.pct("front");
            let debt_cap = gross_m * v.pct("back") - v.get("debts");
            let budget = housing_cap.min(debt_cap);
            if budget <= 0.0 {
                return Output::warning_only(
                    "Your current debt payments already use up the allowed debt-to-income ratio.",
                );
            }
            let n = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let r = v.pct("rate") / 12.0;
            let f = pmt(r, n, 1.0);
            let tax_m = v.pct("tax") / 12.0;
            let ins_m = v.get("ins") / 12.0;
            let down = v.get("down");
            // budget = (price - down) * f + price * tax_m + ins_m
            let mut price = (budget - ins_m + down * f) / (f + tax_m);
            if price < down {
                price = ((budget - ins_m) / tax_m.max(1e-9)).min(down).max(0.0);
            }
            let loan = (price - down).max(0.0);
            let pi = pmt(r, n, loan);
            let mut o = Output::default();
            o.primary("Home price you can afford", money0(price));
            o.row("Loan amount", money0(loan))
                .row("Monthly payment (PITI)", money(pi + price * tax_m + ins_m))
                .row("Principal & interest", money(pi))
                .row("Property tax", money(price * tax_m))
                .row("Home insurance", money(ins_m))
                .row(
                    "Limited by",
                    if housing_cap <= debt_cap {
                        "Housing ratio"
                    } else {
                        "Total debt ratio"
                    },
                );
            if loan > 0.0 && down / price < 0.2 {
                o.note("Down payment is under 20%, so lenders will likely add PMI, which lowers this figure a little.");
            }
            o
        },
    }
}

pub(super) fn refinance() -> Calc {
    Calc {
        name: "Mortgage Refinance",
        blurb: "Monthly savings, closing costs and how long until a refinance pays for itself.",
        fields: vec![
            Field::money("balance", "Current loan balance", 300_000.0),
            rate("cur_rate", "Current interest rate", 7.0, 12.0),
            Field::slider(
                "remaining",
                "Years remaining",
                Unit::Years,
                27.0,
                1.0,
                30.0,
                1.0,
            ),
            rate("new_rate", "New interest rate", 6.0, 12.0),
            Field::choice("term", "New loan term", &MORTGAGE_TERMS, 3),
            Field::money("costs", "Closing costs", 6_000.0),
            Field::choice("roll", "Closing costs", &["Pay upfront", "Add to loan"], 0),
        ],
        compute: |v| {
            let bal = v.get("balance");
            let n1 = v.get("remaining") * 12.0;
            let p1 = pmt(v.pct("cur_rate") / 12.0, n1, bal);
            let costs = v.get("costs");
            let rolled = v.idx("roll") == 1;
            let new_loan = bal + if rolled { costs } else { 0.0 };
            let n2 = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let p2 = pmt(v.pct("new_rate") / 12.0, n2, new_loan);
            let save = p1 - p2;
            let int1 = p1 * n1 - bal;
            let int2 = p2 * n2 - new_loan;
            let total1 = p1 * n1;
            let total2 = p2 * n2 + if rolled { 0.0 } else { costs };
            let mut o = Output::default();
            if save > 0.0 {
                o.primary("Monthly savings", money(save));
            } else {
                o.primary("Payment increases by", money(-save));
            }
            o.row("New payment", money(p2))
                .row("Current payment", money(p1));
            if save > 0.0 && costs > 0.0 {
                o.row("Break-even", duration((costs / save).ceil() as u32));
            }
            o.row("Interest left on current loan", money0(int1))
                .row("Interest on new loan", money0(int2))
                .row(
                    if total2 <= total1 {
                        "Lifetime savings"
                    } else {
                        "Lifetime extra cost"
                    },
                    money0((total1 - total2).abs()),
                );
            if n2 > n1 {
                o.note("The new loan runs longer than what's left on your current one, so part of the lower payment comes from stretching the term.");
            }
            o
        },
    }
}

pub(super) fn extra_payments() -> Calc {
    Calc {
        name: "Pay Off Early",
        blurb: "How extra monthly, yearly or one-time payments shorten your mortgage.",
        fields: vec![
            Field::money("balance", "Loan balance", 300_000.0),
            rate("rate", "Interest rate", 6.5, 12.0),
            Field::slider(
                "years",
                "Years remaining",
                Unit::Years,
                30.0,
                1.0,
                40.0,
                1.0,
            ),
            Field::money("monthly", "Extra each month", 200.0).quick(&[25.0, 50.0, 100.0, 500.0]),
            Field::money("yearly", "Extra once a year", 0.0)
                .quick(&[100.0, 500.0, 1_000.0, 5_000.0]),
            Field::money("lump", "One-time extra payment", 0.0)
                .quick(&[1_000.0, 5_000.0, 10_000.0]),
            Field::number(
                "lump_month",
                "One-time payment in month",
                Unit::Months,
                12.0,
            )
            .min(1.0)
            .max(480.0),
        ],
        compute: |v| {
            let bal0 = v.get("balance");
            let r = v.pct("rate") / 12.0;
            let n = v.get("years") * 12.0;
            let p = pmt(r, n, bal0);
            let base = amortize(bal0, r, p, 0.0, n as u32 + 1, 12);
            let (mut bal, mut month, mut interest) = (bal0, 0u32, 0.0);
            let mut rows = vec![];
            let (mut yp, mut yi) = (0.0, 0.0);
            while bal > 0.005 && month < 600 {
                month += 1;
                let int = bal * r;
                let mut pay = p + v.get("monthly");
                if month % 12 == 0 {
                    pay += v.get("yearly");
                }
                if month == v.get("lump_month").round() as u32 {
                    pay += v.get("lump");
                }
                let pay = pay.min(bal + int);
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
            let mut o = Output::default();
            o.primary("Interest saved", money0(base.total_interest - interest));
            o.row("Paid off in", duration(month))
                .row("Time saved", duration(base.periods.saturating_sub(month)))
                .row("Regular payment", money(p))
                .row("Total interest (with extra)", money0(interest))
                .row("Total interest (no extra)", money0(base.total_interest));
            o.table = Some(Table {
                title: "Payoff by year".into(),
                headers: vec![
                    "Year".into(),
                    "Principal".into(),
                    "Interest".into(),
                    "Balance".into(),
                ],
                rows,
            });
            o
        },
    }
}

pub(super) fn rent_vs_buy() -> Calc {
    Calc {
        name: "Rent vs. Buy",
        blurb: "Net cost of buying versus renting over the years you plan to stay.",
        fields: vec![
            Field::slider(
                "years",
                "Years you'll stay",
                Unit::Years,
                7.0,
                1.0,
                30.0,
                1.0,
            ),
            Field::money("price", "Home price", 400_000.0),
            Field::percent("down", "Down payment", 20.0, 0.0, 60.0, 0.5),
            rate("rate", "Mortgage rate", 6.5, 12.0),
            Field::choice("term", "Loan term", &MORTGAGE_TERMS, 3),
            Field::percent("closing", "Closing costs", 3.0, 0.0, 6.0, 0.25),
            Field::percent("tax", "Property tax (yearly)", 1.1, 0.0, 4.0, 0.05),
            Field::money("ins", "Home insurance (yearly)", 1_800.0),
            Field::percent("maint", "Maintenance (yearly)", 1.0, 0.0, 4.0, 0.1)
                .help("Percent of home value"),
            Field::percent("appr", "Home appreciation", 3.0, -5.0, 10.0, 0.25),
            Field::percent("sell", "Selling costs", 6.0, 0.0, 10.0, 0.5),
            Field::money("rent", "Monthly rent", 2_200.0),
            Field::percent("rent_up", "Yearly rent increase", 3.0, 0.0, 10.0, 0.25),
            Field::money("rins", "Renter's insurance (yearly)", 200.0),
            Field::percent("ret", "Investment return", 6.0, 0.0, 12.0, 0.25)
                .help("What the down payment would earn if you rent"),
        ],
        compute: |v| {
            let price = v.get("price");
            let down = price * v.pct("down");
            let closing = price * v.pct("closing");
            let loan = price - down;
            let n = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let r = v.pct("rate") / 12.0;
            let p = pmt(r, n, loan);
            let horizon = v.get("years").round() as u32;
            let (mut buy_out, mut rent_out) = (down + closing, 0.0);
            let mut rent = v.get("rent");
            let mut rows = vec![];
            let mut breakeven = None;
            let (mut buy_net, mut rent_net) = (0.0, 0.0);
            for y in 1..=30u32 {
                let value_start = price * (1.0 + v.pct("appr")).powi(y as i32 - 1);
                let months_paid = ((y - 1) * 12) as f64;
                let this_year_pi = 12.0_f64.min((n - months_paid).max(0.0)) * p;
                buy_out +=
                    this_year_pi + value_start * (v.pct("tax") + v.pct("maint")) + v.get("ins");
                rent_out += rent * 12.0 + v.get("rins");
                rent *= 1.0 + v.pct("rent_up");
                let value = price * (1.0 + v.pct("appr")).powi(y as i32);
                let bal = balance_after(loan, r, p, (y as f64 * 12.0).min(n));
                let equity = value * (1.0 - v.pct("sell")) - bal;
                let b = buy_out - equity;
                let gain = (down + closing) * ((1.0 + v.pct("ret")).powi(y as i32) - 1.0);
                let rn = rent_out - gain;
                if breakeven.is_none() && b < rn {
                    breakeven = Some(y);
                }
                if y <= horizon.max(10) {
                    rows.push(vec![
                        y.to_string(),
                        money0(b),
                        money0(rn),
                        if b < rn { "Buy".into() } else { "Rent".into() },
                    ]);
                }
                if y == horizon {
                    buy_net = b;
                    rent_net = rn;
                }
            }
            let mut o = Output::default();
            if buy_net < rent_net {
                o.primary(
                    format!("Buying saves over {horizon} yrs"),
                    money0(rent_net - buy_net),
                );
            } else {
                o.primary(
                    format!("Renting saves over {horizon} yrs"),
                    money0(buy_net - rent_net),
                );
            }
            o.row("Net cost of buying", money0(buy_net))
                .row("Net cost of renting", money0(rent_net))
                .row("Mortgage payment (P&I)", money(p))
                .row(
                    "Buying pays off after",
                    breakeven
                        .map(|y| format!("{y} yr{}", if y == 1 { "" } else { "s" }))
                        .unwrap_or_else(|| "30+ yrs".into()),
                );
            o.note("Buying = everything you pay minus your equity after selling. Renting = rent paid minus what your down payment and closing costs would have earned invested.");
            o.table = Some(Table {
                title: "Net cost by year".into(),
                headers: vec!["Year".into(), "Buy".into(), "Rent".into(), "Cheaper".into()],
                rows,
            });
            o
        },
    }
}

pub(super) fn mortgage_points() -> Calc {
    Calc {
        name: "Mortgage Points",
        blurb: "Is buying discount points worth it? Cost, monthly savings and break-even.",
        fields: vec![
            Field::money("loan", "Loan amount", 320_000.0),
            rate("rate", "Rate with no points", 6.75, 12.0),
            Field::slider("points", "Points to buy", Unit::Plain, 1.0, 0.0, 4.0, 0.25)
                .help("1 point = 1% of the loan amount"),
            Field::percent("cut", "Rate cut per point", 0.25, 0.0, 0.5, 0.025),
            Field::choice("term", "Loan term", &MORTGAGE_TERMS, 3),
            Field::slider(
                "keep",
                "Years you'll keep the loan",
                Unit::Years,
                10.0,
                1.0,
                30.0,
                1.0,
            ),
        ],
        compute: |v| {
            let loan = v.get("loan");
            let pts = v.get("points");
            let cost = loan * pts / 100.0;
            let r0 = v.pct("rate");
            let r1 = (r0 - pts * v.pct("cut")).max(0.0);
            let n = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let p0 = pmt(r0 / 12.0, n, loan);
            let p1 = pmt(r1 / 12.0, n, loan);
            let k = (v.get("keep") * 12.0).min(n);
            let save = p0 - p1;
            let bal_diff =
                balance_after(loan, r0 / 12.0, p0, k) - balance_after(loan, r1 / 12.0, p1, k);
            let net = save * k + bal_diff - cost;
            let mut o = Output::default();
            if pts <= 0.0 {
                return Output::warning_only("Choose how many points to buy.");
            }
            if net >= 0.0 {
                o.primary(
                    format!("Points save you over {} yrs", v.get("keep")),
                    money0(net),
                );
            } else {
                o.primary(
                    format!("Points cost you over {} yrs", v.get("keep")),
                    money0(-net),
                );
            }
            o.row("Cost of points", money0(cost))
                .row("New rate", pct(r1 * 100.0))
                .row("Monthly savings", money(save))
                .row("Payment without points", money(p0))
                .row("Payment with points", money(p1));
            if save > 0.0 {
                o.row("Break-even", duration((cost / save).ceil() as u32));
            }
            o.note("Includes the lower balance you'd owe when you sell or refinance.");
            o
        },
    }
}
