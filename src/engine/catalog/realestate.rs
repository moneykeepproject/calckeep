//! Real Estate calculators

use super::*;

pub(super) fn rental_property() -> Calc {
    Calc {
        name: "Rental Property Cash Flow",
        blurb: "Monthly cash flow, cap rate and cash-on-cash return for a rental property.",
        fields: vec![
            Field::money("price", "Purchase price", 300_000.0),
            Field::percent("down", "Down payment", 25.0, 0.0, 100.0, 0.5),
            rate("rate", "Mortgage rate", 7.0, 12.0),
            Field::choice("term", "Loan term", &MORTGAGE_TERMS, 3),
            Field::money("closing", "Closing costs & repairs", 9_000.0),
            Field::money("rent", "Monthly rent", 2_400.0),
            Field::percent("vac", "Vacancy", 5.0, 0.0, 25.0, 1.0),
            Field::money("tax", "Property tax (yearly)", 3_600.0),
            Field::money("ins", "Insurance (yearly)", 1_500.0),
            Field::percent("maint", "Maintenance", 8.0, 0.0, 25.0, 1.0).help("Percent of rent"),
            Field::percent("mgmt", "Property management", 8.0, 0.0, 15.0, 1.0)
                .help("Percent of rent; 0 if you manage it"),
            Field::money("hoa", "HOA & other (monthly)", 0.0),
        ],
        compute: |v| {
            let price = v.get("price");
            let down = price * v.pct("down");
            let loan = price - down;
            let n = MORTGAGE_YEARS[v.idx("term").min(3)] * 12.0;
            let debt_m = pmt(v.pct("rate") / 12.0, n, loan);
            let rent = v.get("rent");
            let income = rent * (1.0 - v.pct("vac"));
            let opex_m = v.get("tax") / 12.0
                + v.get("ins") / 12.0
                + rent * (v.pct("maint") + v.pct("mgmt"))
                + v.get("hoa");
            let noi = (income - opex_m) * 12.0;
            let cash_flow = income - opex_m - debt_m;
            let invested = down + v.get("closing");
            let mut o = Output::default();
            o.primary("Monthly cash flow", money(cash_flow));
            o.row(
                "Cash-on-cash return",
                pct2(if invested > 0.0 {
                    cash_flow * 12.0 / invested * 100.0
                } else {
                    0.0
                }),
            )
            .row("Cap rate", pct2(noi / price.max(1.0) * 100.0))
            .row("Net operating income (yearly)", money0(noi))
            .row("Mortgage payment", money(debt_m))
            .row("Operating expenses (monthly)", money(opex_m))
            .row("Cash invested", money0(invested));
            if debt_m > 0.0 {
                o.row("Debt coverage ratio", trim(noi / (debt_m * 12.0), 2));
            }
            o.row(
                "Rent-to-price (1% rule)",
                pct2(rent / price.max(1.0) * 100.0),
            );
            if cash_flow < 0.0 {
                o.warn("This property loses money each month at these numbers.");
            }
            o.note("Lenders usually want a debt coverage ratio of 1.2 or higher.");
            o
        },
    }
}

pub(super) fn cap_rate() -> Calc {
    Calc {
        name: "Cap Rate & Property Value",
        blurb: "Find a property's cap rate from its price, or its value from a target cap rate.",
        fields: vec![
            Field::choice("mode", "Calculate", &["Cap rate", "Property value"], 0),
            Field::money("price", "Purchase price", 500_000.0).show_if("mode", 0.0),
            Field::percent("target", "Target cap rate", 6.0, 1.0, 15.0, 0.1).show_if("mode", 1.0),
            Field::money("gross", "Gross yearly rent", 60_000.0),
            Field::percent("vac", "Vacancy", 5.0, 0.0, 25.0, 1.0),
            Field::money("opex", "Operating expenses (yearly)", 20_000.0)
                .help("Taxes, insurance, maintenance, management; not the mortgage"),
        ],
        compute: |v| {
            let noi = v.get("gross") * (1.0 - v.pct("vac")) - v.get("opex");
            let mut o = Output::default();
            if v.idx("mode") == 0 {
                let price = v.get("price").max(1.0);
                o.primary("Cap rate", pct2(noi / price * 100.0));
                o.row("Net operating income", money0(noi)).row(
                    "Gross rent multiplier",
                    trim(price / v.get("gross").max(1.0), 2),
                );
            } else {
                let value = noi / v.pct("target").max(1e-6);
                o.primary("Property value", money0(value));
                o.row("Net operating income", money0(noi)).row(
                    "Gross rent multiplier",
                    trim(value / v.get("gross").max(1.0), 2),
                );
            }
            o.row(
                "Expense ratio",
                pct2(v.get("opex") / v.get("gross").max(1.0) * 100.0),
            );
            o.note("Cap rate = net operating income ÷ price. A higher cap rate means more income per dollar, usually with more risk.");
            o
        },
    }
}
