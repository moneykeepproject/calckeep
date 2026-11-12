//! Auto calculators

use super::*;

pub(super) const LEASE_TERMS: [&str; 4] = ["24 mo", "36 mo", "39 mo", "48 mo"];
pub(super) const LEASE_MONTHS: [f64; 4] = [24.0, 36.0, 39.0, 48.0];

/// Returns (monthly with tax, depreciation, finance charge, tax per month, residual, adj cap cost)
#[allow(clippy::too_many_arguments)] // one argument per lease input, as the formula names them
pub(super) fn lease_payment(
    msrp: f64,
    price: f64,
    fees: f64,
    down: f64,
    trade: f64,
    residual_pct: f64,
    apr: f64,
    n: f64,
    tax: f64,
) -> (f64, f64, f64, f64, f64, f64) {
    let adj = price + fees - down - trade;
    let residual = msrp * residual_pct;
    let mf = apr / 2400.0;
    let dep = (adj - residual) / n;
    let fin = (adj + residual) * mf;
    let base = dep + fin;
    let t = base * tax;
    (base + t, dep, fin, t, residual, adj)
}

pub(super) fn auto_loan() -> Calc {
    Calc {
        name: "Auto Loan Payment",
        blurb: "Monthly car payment including sales tax, fees, down payment and trade-in.",
        fields: vec![
            Field::money("price", "Vehicle price", 35_000.0),
            Field::money("down", "Down payment", 5_000.0),
            Field::money("trade", "Trade-in value", 0.0),
            rate("rate", "Interest rate (APR)", 7.0, 25.0),
            Field::choice("term", "Loan term", &AUTO_TERMS, 2),
            Field::percent("tax", "Sales tax", 6.0, 0.0, 12.0, 0.25)
                .help("Applied to price minus trade-in"),
            Field::money("fees", "Title, registration & fees", 1_000.0),
        ],
        compute: |v| {
            let price = v.get("price");
            let trade = v.get("trade");
            let tax = (price - trade).max(0.0) * v.pct("tax");
            let loan = (price + tax + v.get("fees") - v.get("down") - trade).max(0.0);
            let n = AUTO_MONTHS[v.idx("term").min(4)];
            let r = v.pct("rate") / 12.0;
            let p = pmt(r, n, loan);
            let a = amortize(loan, r, p, 0.0, n as u32 + 1, 12);
            let mut o = Output::default();
            o.primary("Monthly payment", money(p));
            o.row("Amount financed", money(loan))
                .row("Sales tax", money(tax))
                .row("Total interest", money(a.total_interest))
                .row("Total of payments", money(a.total_paid))
                .row(
                    "Total cost (with down & trade)",
                    money(a.total_paid + v.get("down") + trade),
                );
            o.table = Some(yearly_table("Payoff by year", &a, "Year"));
            o
        },
    }
}

pub(super) fn auto_affordability() -> Calc {
    Calc {
        name: "Car Affordability",
        blurb: "The most car you can buy for the monthly payment you want.",
        fields: vec![
            Field::money("budget", "Monthly payment budget", 600.0),
            Field::money("down", "Down payment", 5_000.0),
            Field::money("trade", "Trade-in value", 0.0),
            rate("rate", "Interest rate (APR)", 7.0, 25.0),
            Field::choice("term", "Loan term", &AUTO_TERMS, 2),
            Field::percent("tax", "Sales tax", 6.0, 0.0, 12.0, 0.25),
            Field::money("fees", "Title, registration & fees", 1_000.0),
        ],
        compute: |v| {
            let n = AUTO_MONTHS[v.idx("term").min(4)];
            let r = v.pct("rate") / 12.0;
            let loan = pv_of_payments(r, n, v.get("budget"));
            let t = v.pct("tax");
            let trade = v.get("trade");
            // loan = price + (price - trade) * t + fees - down - trade
            let price =
                ((loan - v.get("fees") + v.get("down") + trade + trade * t) / (1.0 + t)).max(0.0);
            let tax = (price - trade).max(0.0) * t;
            let mut o = Output::default();
            o.primary("Vehicle price you can afford", money0(price));
            o.row("Amount financed", money(loan))
                .row("Sales tax", money(tax))
                .row("Total interest", money(v.get("budget") * n - loan))
                .row("Total of payments", money(v.get("budget") * n));
            o
        },
    }
}

pub(super) fn auto_lease() -> Calc {
    Calc {
        name: "Lease Payment",
        blurb: "Monthly lease payment from MSRP, negotiated price, residual and money factor.",
        fields: vec![
            Field::money("msrp", "MSRP (sticker price)", 40_000.0),
            Field::money("price", "Negotiated price", 37_500.0),
            Field::money("down", "Down payment", 2_000.0),
            Field::money("trade", "Trade-in value", 0.0),
            Field::percent("residual", "Residual value", 58.0, 30.0, 80.0, 1.0)
                .help("Percent of MSRP at lease end"),
            Field::percent("apr", "Interest rate (APR)", 5.0, 0.0, 15.0, 0.1)
                .help("Money factor = APR ÷ 2400"),
            Field::choice("term", "Lease term", &LEASE_TERMS, 1),
            Field::percent("tax", "Sales tax", 6.0, 0.0, 12.0, 0.25)
                .help("Applied to each monthly payment"),
            Field::money("fees", "Acquisition & dealer fees", 995.0),
        ],
        compute: |v| {
            let n = LEASE_MONTHS[v.idx("term").min(3)];
            let (mo, dep, fin, t, residual, adj) = lease_payment(
                v.get("msrp"),
                v.get("price"),
                v.get("fees"),
                v.get("down"),
                v.get("trade"),
                v.pct("residual"),
                v.get("apr"),
                n,
                v.pct("tax"),
            );
            let mut o = Output::default();
            if adj < residual {
                o.warn("The adjusted price is below the residual value; check the down payment and trade-in.");
            }
            o.primary("Monthly payment", money(mo.max(0.0)));
            o.row("Depreciation portion", money(dep))
                .row("Finance (rent) charge", money(fin))
                .row("Sales tax", money(t))
                .row("Money factor", format!("{:.5}", v.get("apr") / 2400.0))
                .row("Adjusted cap cost", money(adj))
                .row("Residual value", money(residual))
                .row(
                    "Total lease cost",
                    money(mo * n + v.get("down") + v.get("trade")),
                );
            o
        },
    }
}

pub(super) fn lease_vs_buy() -> Calc {
    Calc {
        name: "Lease vs. Buy",
        blurb: "Compare the net cost of leasing against financing and selling at lease end.",
        fields: vec![
            Field::money("price", "Vehicle price", 40_000.0),
            Field::money("down", "Down payment", 3_000.0),
            Field::percent("tax", "Sales tax", 6.0, 0.0, 12.0, 0.25),
            rate("loan_rate", "Loan rate (APR)", 7.0, 20.0),
            Field::choice("loan_term", "Loan term", &["48 mo", "60 mo", "72 mo"], 1),
            Field::percent("lease_apr", "Lease rate (APR)", 5.0, 0.0, 15.0, 0.1),
            Field::choice("lease_term", "Lease term", &["24 mo", "36 mo", "48 mo"], 1),
            Field::percent("residual", "Value at lease end", 58.0, 30.0, 80.0, 1.0)
                .help("Residual / resale value as % of price"),
            Field::money("fees", "Lease fees", 995.0),
        ],
        compute: |v| {
            let price = v.get("price");
            let down = v.get("down");
            let t = v.pct("tax");
            let nl: f64 = [24.0, 36.0, 48.0][v.idx("lease_term").min(2)];
            let nb = [48.0, 60.0, 72.0][v.idx("loan_term").min(2)];
            let rb = v.pct("loan_rate") / 12.0;
            // Buy
            let loan = (price * (1.0 + t) - down).max(0.0);
            let pay_b = pmt(rb, nb, loan);
            let k = f64::min(nl, nb);
            let bal = balance_after(loan, rb, pay_b, k);
            let value = price * v.pct("residual");
            let buy_net = down + pay_b * k + bal - value;
            // Lease
            let (pay_l, ..) = lease_payment(
                price,
                price,
                v.get("fees"),
                down,
                0.0,
                v.pct("residual"),
                v.get("lease_apr"),
                nl,
                t,
            );
            let lease_net = down + pay_l * nl;
            let mut o = Output::default();
            let diff = (buy_net - lease_net).abs();
            if lease_net < buy_net {
                o.primary("Leasing costs less by", money0(diff));
            } else {
                o.primary("Buying costs less by", money0(diff));
            }
            o.row("Lease monthly payment", money(pay_l))
                .row(format!("Lease cost over {} mo", nl), money0(lease_net))
                .row("Loan monthly payment", money(pay_b))
                .row(format!("Buy net cost over {} mo", nl), money0(buy_net))
                .row("Car value at lease end", money0(value))
                .row("Loan balance at lease end", money0(bal));
            o.note("Buy net cost = down payment + loan payments + remaining loan balance − car's resale value.");
            o
        },
    }
}

pub(super) fn rebate_vs_apr() -> Calc {
    Calc {
        name: "Cash Rebate vs. Low APR",
        blurb: "Take the dealer's cash back or the special financing rate? Compare total cost.",
        fields: vec![
            Field::money("price", "Vehicle price", 35_000.0),
            Field::money("down", "Down payment", 3_000.0),
            Field::money("rebate", "Cash rebate", 3_000.0),
            Field::percent("dealer", "Dealer special APR", 0.9, 0.0, 15.0, 0.1),
            Field::percent("bank", "Your bank / credit union APR", 6.5, 0.0, 20.0, 0.1),
            Field::choice("term", "Loan term", &AUTO_TERMS, 2),
        ],
        compute: |v| {
            let n = AUTO_MONTHS[v.idx("term").min(4)];
            let base = (v.get("price") - v.get("down")).max(0.0);
            let loan_a = (base - v.get("rebate")).max(0.0);
            let pa = pmt(v.pct("bank") / 12.0, n, loan_a);
            let pb = pmt(v.pct("dealer") / 12.0, n, base);
            let (ta, tb) = (pa * n, pb * n);
            let mut o = Output::default();
            if ta <= tb {
                o.primary("Take the rebate — saves", money0(tb - ta));
            } else {
                o.primary("Take the low APR — saves", money0(ta - tb));
            }
            o.row("Rebate: monthly payment", money(pa))
                .row("Rebate: total of payments", money0(ta))
                .row("Low APR: monthly payment", money(pb))
                .row("Low APR: total of payments", money0(tb));
            // Bank rate at which both options cost the same
            if let Some(r) = solve(|r| pmt(r / 12.0, n, loan_a) * n - tb, 0.0, 0.5) {
                o.note(format!(
                    "The rebate wins as long as your bank's rate is below {}.",
                    pct2(r * 100.0)
                ));
            }
            o
        },
    }
}

pub(super) fn auto_refinance() -> Calc {
    Calc {
        name: "Auto Refinance",
        blurb: "Savings from refinancing your current car loan at a lower rate.",
        fields: vec![
            Field::money("balance", "Current loan balance", 22_000.0),
            Field::percent("cur", "Current APR", 9.0, 0.0, 25.0, 0.25),
            Field::slider(
                "left",
                "Months remaining",
                Unit::Months,
                48.0,
                6.0,
                84.0,
                1.0,
            ),
            Field::percent("new", "New APR", 6.0, 0.0, 25.0, 0.25),
            Field::choice("term", "New loan term", &AUTO_TERMS, 1),
            Field::money("fees", "Refinance fees", 200.0),
        ],
        compute: |v| {
            let bal = v.get("balance");
            let n1 = v.get("left").round();
            let p1 = pmt(v.pct("cur") / 12.0, n1, bal);
            let n2 = AUTO_MONTHS[v.idx("term").min(4)];
            let p2 = pmt(v.pct("new") / 12.0, n2, bal);
            let (t1, t2) = (p1 * n1, p2 * n2 + v.get("fees"));
            let mut o = Output::default();
            if t2 <= t1 {
                o.primary("Total savings", money0(t1 - t2));
            } else {
                o.primary("Refinancing costs more by", money0(t2 - t1));
            }
            o.row("New monthly payment", money(p2))
                .row("Current monthly payment", money(p1))
                .row("Monthly difference", money(p1 - p2))
                .row("Interest left on current loan", money0(p1 * n1 - bal))
                .row("Interest on new loan", money0(p2 * n2 - bal));
            if n2 > n1 {
                o.note("The new term is longer than what's left, which lowers the payment but adds months of payments.");
            }
            o
        },
    }
}

pub(super) fn lease_buyout() -> Calc {
    Calc {
        name: "Lease Buyout vs. Return",
        blurb: "Should you buy your car at lease end? Compare the buyout with its market value.",
        fields: vec![
            Field::money("buyout", "Buyout price (residual)", 23_000.0),
            Field::money("opt_fee", "Purchase option fee", 300.0),
            Field::percent("tax", "Sales tax", 6.0, 0.0, 12.0, 0.25),
            Field::money("reg", "Title & registration", 400.0),
            Field::money("market", "Car's market value", 26_000.0)
                .help("Check trade-in or private-sale value"),
            Field::money("dispo", "Disposition fee if returned", 395.0),
            Field::money("charges", "Mileage & wear charges if returned", 0.0),
        ],
        compute: |v| {
            let buy = v.get("buyout") * (1.0 + v.pct("tax")) + v.get("opt_fee") + v.get("reg");
            let equity = v.get("market") - buy;
            let avoided = v.get("dispo") + v.get("charges");
            let advantage = equity + avoided;
            let mut o = Output::default();
            if advantage >= 0.0 {
                o.primary("Buying comes out ahead by", money0(advantage));
            } else {
                o.primary("Returning comes out ahead by", money0(-advantage));
            }
            o.row("Total cost to buy", money0(buy))
                .row("Market value", money0(v.get("market")))
                .row(
                    if equity >= 0.0 {
                        "Equity in the car"
                    } else {
                        "Buyout over market value"
                    },
                    money0(equity.abs()),
                )
                .row("Return fees you'd avoid", money0(avoided));
            if equity > 0.0 {
                o.note("With positive equity you could also buy it out and sell it, or use it as a trade-in.");
            }
            o
        },
    }
}

pub(super) fn mileage_overage() -> Calc {
    Calc {
        name: "Lease Mileage",
        blurb: "Cost of going over your mileage allowance, and whether to buy miles up front.",
        fields: vec![
            Field::slider(
                "allowed",
                "Miles allowed per year",
                Unit::Count,
                12_000.0,
                5_000.0,
                20_000.0,
                1_000.0,
            ),
            Field::slider(
                "expected",
                "Miles you'll drive per year",
                Unit::Count,
                15_000.0,
                3_000.0,
                30_000.0,
                500.0,
            ),
            Field::choice("term", "Lease term", &LEASE_TERMS, 1),
            Field::money("rate", "Overage charge per mile", 0.25).quick(&[0.01, 0.05]),
            Field::money("prebuy", "Price per mile if bought up front", 0.15).quick(&[0.01, 0.05]),
        ],
        compute: |v| {
            let years = LEASE_MONTHS[v.idx("term").min(3)] / 12.0;
            let over = (v.get("expected") - v.get("allowed")) * years;
            let mut o = Output::default();
            if over <= 0.0 {
                o.primary("Miles to spare", count(-over));
                o.note("You're within the allowance. Some leases offer a lower payment for fewer miles.");
                return o;
            }
            let at_end = over * v.get("rate");
            let up_front = over * v.get("prebuy");
            o.primary("Overage charge at turn-in", money0(at_end));
            o.row("Extra miles", count(over))
                .row("Cost if bought up front", money0(up_front))
                .row("Savings from buying up front", money0(at_end - up_front))
                .row(
                    "Overage cost per month of lease",
                    money(at_end / (years * 12.0)),
                );
            o
        },
    }
}
