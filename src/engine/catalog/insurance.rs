//! Insurance calculators

use super::*;

pub(super) fn life_insurance() -> Calc {
    Calc {
        name: "Life Insurance Needs",
        blurb: "Estimate how much coverage your family would need.",
        fields: vec![
            Field::money("income", "Annual income to replace", 75_000.0),
            Field::slider(
                "years",
                "Years of support",
                Unit::Years,
                10.0,
                1.0,
                30.0,
                1.0,
            ),
            Field::money("debts", "Mortgage & other debts", 250_000.0),
            Field::money("college", "College funding", 100_000.0),
            Field::money("final", "Final expenses", 15_000.0),
            Field::money("savings", "Savings & investments", 50_000.0),
            Field::money("existing", "Existing life insurance", 0.0),
        ],
        compute: |v| {
            let income_need = v.get("income") * v.get("years");
            let gross = income_need + v.get("debts") + v.get("college") + v.get("final");
            let have = v.get("savings") + v.get("existing");
            let need = (gross - have).max(0.0);
            let mut o = Output::default();
            o.primary("Coverage needed", money0(need));
            o.row("Income replacement", money0(income_need))
                .row("Debts", money0(v.get("debts")))
                .row("College", money0(v.get("college")))
                .row("Final expenses", money0(v.get("final")))
                .row("Less savings & existing coverage", money0(-have))
                .row("Rule of thumb (10× income)", money0(v.get("income") * 10.0));
            o
        },
    }
}

pub(super) fn deductible_compare() -> Calc {
    Calc {
        name: "Deductible Comparison",
        blurb: "Is a higher deductible worth the lower premium? Find the break-even.",
        fields: vec![
            Field::money("prem_a", "Plan A monthly premium", 150.0),
            Field::money("ded_a", "Plan A deductible", 500.0),
            Field::money("prem_b", "Plan B monthly premium", 110.0),
            Field::money("ded_b", "Plan B deductible", 1_500.0),
            Field::slider(
                "claims",
                "Expected claims per year",
                Unit::Plain,
                0.3,
                0.0,
                3.0,
                0.1,
            )
            .help("0.3 = about one claim every 3 years"),
        ],
        compute: |v| {
            let (pa, da, pb, db) = (
                v.get("prem_a") * 12.0,
                v.get("ded_a"),
                v.get("prem_b") * 12.0,
                v.get("ded_b"),
            );
            let c = v.get("claims");
            let cost_a = pa + c * da;
            let cost_b = pb + c * db;
            let mut o = Output::default();
            if (cost_a - cost_b).abs() < 0.5 {
                o.primary("Expected cost", "About the same");
            } else if cost_a < cost_b {
                o.primary("Plan A saves (expected/yr)", money0(cost_b - cost_a));
            } else {
                o.primary("Plan B saves (expected/yr)", money0(cost_a - cost_b));
            }
            o.row("Plan A yearly premium", money0(pa))
                .row("Plan B yearly premium", money0(pb))
                .row("Plan A expected yearly cost", money0(cost_a))
                .row("Plan B expected yearly cost", money0(cost_b));
            // Break-even claim frequency
            let prem_saving = (pa - pb).abs();
            let extra_per_claim = (da - db).abs();
            let high_ded_cheaper_premium = (db > da && pb < pa) || (da > db && pa < pb);
            if high_ded_cheaper_premium && extra_per_claim > 0.0 {
                let breakeven = prem_saving / extra_per_claim;
                o.row("Break-even claims per year", trim(breakeven, 2));
                if breakeven > 0.0 {
                    o.note(format!(
                        "The higher deductible pays off if you file a claim less than about once every {} years.",
                        trim(1.0 / breakeven, 1)
                    ));
                }
            } else if (da - db).abs() > 0.0 {
                o.note("One plan has both the lower premium and the lower deductible, so it's cheaper either way.");
            }
            o.note("Assumes every claim is larger than the deductible.");
            o
        },
    }
}

fn out_of_pocket(spend: f64, ded: f64, coins: f64, oop_max: f64) -> f64 {
    let raw = if spend <= ded {
        spend
    } else {
        ded + coins * (spend - ded)
    };
    raw.min(oop_max).max(0.0)
}

pub(super) fn health_plan() -> Calc {
    Calc {
        name: "Health Plan Comparison",
        blurb: "Compare two health plans, including a high-deductible plan with an HSA.",
        fields: vec![
            Field::money("spend", "Expected medical spending (yearly)", 3_000.0)
                .help("Total billed amount after network discounts"),
            Field::money("pa", "Plan A monthly premium", 250.0),
            Field::money("da", "Plan A deductible", 1_000.0),
            Field::percent("ca", "Plan A coinsurance", 20.0, 0.0, 50.0, 5.0),
            Field::money("ma", "Plan A out-of-pocket max", 5_000.0),
            Field::money("pb", "Plan B monthly premium", 150.0),
            Field::money("db", "Plan B deductible", 3_400.0),
            Field::percent("cb", "Plan B coinsurance", 20.0, 0.0, 50.0, 5.0),
            Field::money("mb", "Plan B out-of-pocket max", 8_500.0),
            Field::money("hsa_er", "Plan B employer HSA contribution", 1_000.0),
            Field::money("hsa_you", "Plan B your HSA contribution", 2_000.0)
                .help("2026 HSA limit: $4,400 self / $8,750 family, employer + you"),
            Field::percent(
                "tax",
                "Your tax rate (for HSA savings)",
                22.0,
                0.0,
                45.0,
                1.0,
            ),
        ],
        compute: |v| {
            let cost_a = |s: f64| {
                v.get("pa") * 12.0 + out_of_pocket(s, v.get("da"), v.pct("ca"), v.get("ma"))
            };
            let hsa_benefit = v.get("hsa_er") + v.get("hsa_you") * v.pct("tax");
            let cost_b = |s: f64| {
                v.get("pb") * 12.0 + out_of_pocket(s, v.get("db"), v.pct("cb"), v.get("mb"))
                    - hsa_benefit
            };
            let s = v.get("spend");
            let (a, b) = (cost_a(s), cost_b(s));
            let mut o = Output::default();
            if (a - b).abs() < 1.0 {
                o.primary("Expected yearly cost", "About the same");
            } else if a < b {
                o.primary("Plan A saves (expected)", money0(b - a));
            } else {
                o.primary("Plan B saves (expected)", money0(a - b));
            }
            o.row("Plan A expected yearly cost", money0(a))
                .row("Plan B expected yearly cost", money0(b))
                .row(
                    "Plan A worst case",
                    money0(v.get("pa") * 12.0 + v.get("ma")),
                )
                .row(
                    "Plan B worst case",
                    money0(v.get("pb") * 12.0 + v.get("mb") - hsa_benefit),
                )
                .row("HSA value (employer + tax savings)", money0(hsa_benefit));
            if v.get("hsa_er") + v.get("hsa_you") > 8_750.0 {
                o.warn("HSA contributions are above the 2026 family limit of $8,750.");
            }
            o.note("Costs = premiums + your share of care. Plan B is credited with the employer's HSA money and the tax you save on your own HSA contributions.");
            let mut levels = vec![0.0, 1_000.0, s, 10_000.0, 25_000.0];
            levels.sort_by(|x, y| x.partial_cmp(y).unwrap());
            levels.dedup();
            o.table = Some(Table {
                title: "Yearly cost by medical spending".into(),
                headers: vec!["Spending".into(), "Plan A".into(), "Plan B".into()],
                rows: levels
                    .into_iter()
                    .map(|x| vec![money0(x), money0(cost_a(x)), money0(cost_b(x))])
                    .collect(),
            });
            o
        },
    }
}

pub(super) fn disability() -> Calc {
    Calc {
        name: "Disability Insurance Needs",
        blurb: "How much monthly disability coverage you need on top of what you have.",
        fields: vec![
            Field::money("income", "Monthly gross income", 6_000.0),
            Field::percent("target", "Income to replace", 60.0, 40.0, 80.0, 5.0)
                .help("Policies usually cover 50–70%"),
            Field::percent(
                "employer",
                "Employer long-term disability",
                0.0,
                0.0,
                70.0,
                5.0,
            )
            .help("Percent of income your employer plan pays"),
            Field::money("other", "Other monthly income if disabled", 0.0)
                .help("Spouse's income, rental income, etc."),
            Field::money("expenses", "Monthly essential expenses", 4_500.0),
        ],
        compute: |v| {
            let inc = v.get("income");
            let target = inc * v.pct("target");
            let existing = inc * v.pct("employer");
            let gap_income = (target - existing).max(0.0);
            let gap_expenses = (v.get("expenses") - v.get("other") - existing).max(0.0);
            let need = gap_income.max(gap_expenses).min(inc * 0.7);
            let mut o = Output::default();
            o.primary("Additional monthly coverage", money0(need));
            o.row("Income replacement goal", money0(target))
                .row("Employer coverage", money0(existing))
                .row("Expense gap if disabled", money0(gap_expenses))
                .row(
                    "Rough yearly premium (1–3% of income)",
                    format!(
                        "{} – {}",
                        money0(inc * 12.0 * 0.01),
                        money0(inc * 12.0 * 0.03)
                    ),
                );
            o.note("Employer-paid benefits are usually taxable; benefits from a policy you pay for with after-tax money usually aren't.");
            o
        },
    }
}
