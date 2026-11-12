//! The calculator catalog. To add a calculator: write a `fn my_calc() -> Calc`
//! and push it into the right category in [`categories`]. Nothing else changes.

pub mod auto;
pub mod business;
pub mod education;
pub mod insurance;
pub mod investments;
pub mod loan;
pub mod mortgage;
pub mod payoff;
pub mod realestate;
pub mod retirement;
pub mod savings;
pub mod tax;

use super::finance::*;
use super::format::*;
use super::{Calc, Category, Field, Output, Table, Unit, Values};
use auto::*;
use business::*;
use education::*;
use insurance::*;
use investments::*;
use loan::*;
use mortgage::*;
use payoff::*;
use realestate::*;
use retirement::*;
use savings::*;
use tax::*;

pub fn categories() -> Vec<Category> {
    vec![
        Category {
            name: "Mortgage",
            subtitle: "Payments, refinance & rent vs. buy",
            icon: 0,
            calcs: vec![
                mortgage_payment(),
                mortgage_affordability(),
                refinance(),
                extra_payments(),
                rent_vs_buy(),
                mortgage_points(),
            ],
        },
        Category {
            name: "Auto Loan",
            subtitle: "Payments, budget & refinance",
            icon: 1,
            calcs: vec![
                auto_loan(),
                auto_affordability(),
                rebate_vs_apr(),
                auto_refinance(),
            ],
        },
        Category {
            name: "Auto Lease",
            subtitle: "Payments, buyout & mileage",
            icon: 2,
            calcs: vec![
                auto_lease(),
                lease_vs_buy(),
                lease_buyout(),
                mileage_overage(),
            ],
        },
        Category {
            name: "Pay Off",
            subtitle: "Credit cards & debt plans",
            icon: 3,
            calcs: vec![
                credit_card_payoff(),
                debt_snowball(),
                debt_consolidation(),
                minimum_payment(),
                payoff_vs_invest(),
            ],
        },
        Category {
            name: "Loan",
            subtitle: "Payments, comparisons & HELOC",
            icon: 4,
            calcs: vec![
                loan_payment(),
                amortization_schedule(),
                loan_comparison(),
                heloc(),
                student_loan(),
            ],
        },
        Category {
            name: "Retirement",
            subtitle: "Savings, Social Security & RMDs",
            icon: 5,
            calcs: vec![
                retirement_savings(),
                k401(),
                social_security(),
                savings_last(),
                roth_vs_traditional(),
                rmd(),
            ],
        },
        Category {
            name: "Savings & CD",
            subtitle: "Goals, emergency fund & CDs",
            icon: 6,
            calcs: vec![cd_calc(), savings_goal(), emergency_fund(), cd_ladder()],
        },
        Category {
            name: "Investments",
            subtitle: "Growth, returns & bonds",
            icon: 7,
            calcs: vec![
                compound_growth(),
                roi(),
                inflation(),
                dividend_reinvestment(),
                bond_yield(),
            ],
        },
        Category {
            name: "Tax",
            subtitle: "Income, paycheck & capital gains",
            icon: 8,
            calcs: vec![
                income_tax(),
                tax_equivalent_yield(),
                paycheck(),
                self_employment(),
                capital_gains(),
            ],
        },
        Category {
            name: "Insurance",
            subtitle: "Life, health & disability",
            icon: 9,
            calcs: vec![
                life_insurance(),
                deductible_compare(),
                health_plan(),
                disability(),
            ],
        },
        Category {
            name: "Business",
            subtitle: "Break-even, pricing & ROI",
            icon: 10,
            calcs: vec![
                break_even(),
                profit_margin(),
                employee_cost(),
                payback_npv(),
                pricing(),
            ],
        },
        Category {
            name: "Real Estate",
            subtitle: "Rental cash flow & cap rate",
            icon: 11,
            calcs: vec![rental_property(), cap_rate()],
        },
        Category {
            name: "Education",
            subtitle: "529 savings & college cost",
            icon: 12,
            calcs: vec![college_529(), college_cost()],
        },
    ]
}

// ---------------------------------------------------------------- shared helpers

pub(super) fn rate(key: &'static str, label: &'static str, default: f64, max: f64) -> Field {
    Field::percent(key, label, default, 0.0, max, 0.125)
}

pub(super) fn yearly_table(title: &str, a: &Amortization, label: &str) -> Table {
    Table {
        title: title.into(),
        headers: vec![
            label.into(),
            "Principal".into(),
            "Interest".into(),
            "Balance".into(),
        ],
        rows: a
            .rows
            .iter()
            .map(|r| {
                vec![
                    r.index.to_string(),
                    money0(r.principal),
                    money0(r.interest),
                    money0(r.balance),
                ]
            })
            .collect(),
    }
}

pub(super) const MORTGAGE_TERMS: [&str; 4] = ["10 yr", "15 yr", "20 yr", "30 yr"];
pub(super) const MORTGAGE_YEARS: [f64; 4] = [10.0, 15.0, 20.0, 30.0];
pub(super) const AUTO_TERMS: [&str; 5] = ["36 mo", "48 mo", "60 mo", "72 mo", "84 mo"];
pub(super) const AUTO_MONTHS: [f64; 5] = [36.0, 48.0, 60.0, 72.0, 84.0];

// ---------------------------------------------------------------- Mortgage

// ---------------------------------------------------------------- Auto loan

// ---------------------------------------------------------------- Auto lease

// ---------------------------------------------------------------- Pay off

// ---------------------------------------------------------------- Loan

// ---------------------------------------------------------------- Retirement

// ---------------------------------------------------------------- Savings & CD

// ---------------------------------------------------------------- Investments

// ---------------------------------------------------------------- Tax

// ---------------------------------------------------------------- Insurance

// ---------------------------------------------------------------- Business

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cat: usize, calc: usize, overrides: &[(&str, f64)]) -> Output {
        let cats = categories();
        let c = &cats[cat].calcs[calc];
        let mut vals = c.defaults();
        for (k, val) in overrides {
            let i = c.fields.iter().position(|f| f.key == *k).unwrap();
            vals[i] = *val;
        }
        c.run(&vals)
    }

    fn value(o: &Output, label: &str) -> String {
        o.rows
            .iter()
            .find(|r| r.label == label)
            .map(|r| r.value.clone())
            .unwrap_or_else(|| panic!("no row {label}: {:#?}", o.rows))
    }

    #[test]
    fn every_calculator_runs_with_defaults_and_extremes() {
        let mut bad = vec![];
        for cat in categories() {
            assert!(!cat.calcs.is_empty());
            for c in &cat.calcs {
                let o = c.run(&c.defaults());
                assert!(!o.rows.is_empty(), "{}", c.name);
                // all minimums / all maximums must not panic or produce NaN text
                for pick in [0, 1] {
                    let vals: Vec<f64> = c
                        .fields
                        .iter()
                        .map(|f| if pick == 0 { f.min } else { f.max.min(1e7) })
                        .collect();
                    let o = c.run(&vals);
                    for r in &o.rows {
                        if r.value.contains("NaN")
                            || r.value.contains("inf")
                            || r.label.contains("NaN")
                        {
                            bad.push(format!(
                                "{} [{}] {:?}",
                                c.name,
                                if pick == 0 { "min" } else { "max" },
                                r
                            ));
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    fn find(name: &str, overrides: &[(&str, f64)]) -> Output {
        let cats = categories();
        let c = cats
            .iter()
            .flat_map(|c| c.calcs.iter())
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no calc {name}"));
        let mut vals = c.defaults();
        for (k, val) in overrides {
            let i = c.fields.iter().position(|f| f.key == *k).unwrap();
            vals[i] = *val;
        }
        c.run(&vals)
    }

    #[test]
    fn social_security_matches_ssa_percentages() {
        // FRA 67: 70% at 62, 124% at 70. FRA 66y10m (born 1959): 70.83% at 62, 125.33% at 70.
        assert!((retirement::ss_factor(62.0, 67.0) - 0.70).abs() < 1e-9);
        assert!((retirement::ss_factor(70.0, 67.0) - 1.24).abs() < 1e-9);
        assert!((retirement::ss_factor(62.0, 66.0 + 10.0 / 12.0) - 0.708333).abs() < 1e-5);
        assert!((retirement::ss_factor(70.0, 66.0 + 10.0 / 12.0) - 1.253333).abs() < 1e-5);
        let o = find("Social Security Claiming Age", &[("claim", 62.0)]);
        assert_eq!(value(&o, "Monthly benefit at 62"), "$1,540");
    }

    #[test]
    fn rmd_matches_irs_example() {
        // $500,000 at age 75 / 24.6 = $20,325.20
        let o = find("Required Minimum Distribution", &[]);
        assert_eq!(value(&o, "RMD this year"), "$20,325.20");
        assert_eq!(retirement::ult_divisor(73), 26.5);
        assert_eq!(retirement::ult_divisor(100), 6.4);
        assert_eq!(retirement::ult_divisor(120), 2.0);
    }

    #[test]
    fn capital_gains_long_term() {
        // single, $70k other taxable income (above the $49,450 0% zone): $15k gain all at 15%
        let o = find("Capital Gains Tax", &[]);
        assert_eq!(value(&o, "Federal (long-term rates)"), "$2,250");
        assert_eq!(value(&o, "Net investment income tax (3.8%)"), "$0");
        // $30k other income: 19,450 at 0%, rest of $15k gain... all 15k fits in 0% zone
        let o = find("Capital Gains Tax", &[("income", 30_000.0)]);
        assert_eq!(value(&o, "Federal (long-term rates)"), "$0");
    }

    #[test]
    fn paycheck_federal_withholding() {
        // 75,000 - 4,500 (401k) - 2,600 (health) - 16,100 std = 51,800 taxable -> $6,108 / 26
        let o = find("Paycheck / Take-Home Pay", &[]);
        assert_eq!(value(&o, "Federal income tax"), "$234.92");
    }

    #[test]
    fn self_employment_tax() {
        // 15.3% x 92.35% x 80,000 = 11,303.64
        let o = find("Self-Employment & Quarterly Tax", &[]);
        assert_eq!(value(&o, "Self-employment tax"), "$11,304");
    }

    #[test]
    fn misc_reference_values() {
        assert_eq!(
            value(
                &find("Bond Yield to Maturity", &[("price", 1000.0)]),
                "Yield to maturity"
            ),
            "5.00%"
        );
        assert_eq!(
            value(&find("True Cost of an Employee", &[]), "Total yearly cost"),
            "$79,302"
        );
        assert_eq!(
            value(&find("Rental Property Cash Flow", &[]), "Cap rate"),
            "5.88%"
        );
        assert_eq!(
            value(&find("Payback, NPV & IRR", &[]), "Payback period"),
            "4.2 yrs"
        );
        assert_eq!(
            value(&find("Markup & Pricing", &[]), "Selling price"),
            "$60.00"
        );
        assert_eq!(
            value(&find("Inflation", &[]), "Cost in 10 years"),
            "$134.39"
        );
        assert_eq!(
            value(&find("Mortgage Points", &[]), "Cost of points"),
            "$3,200"
        );
        assert_eq!(
            value(&find("Savings Goal", &[]), "Monthly savings needed"),
            "$465.38"
        );
        let o = find("Cash Rebate vs. Low APR", &[]);
        assert_eq!(value(&o, "Rebate: total of payments"), "$34,045");
        assert_eq!(value(&o, "Low APR: total of payments"), "$32,737");
    }

    #[test]
    fn conditional_fields() {
        let cats = categories();
        let c = cats
            .iter()
            .flat_map(|c| c.calcs.iter())
            .find(|c| c.name == "Debt Consolidation")
            .unwrap();
        let mut vals = c.defaults();
        assert_eq!(c.visible(&vals).len(), 1 + 3 * 3 + 3); // 3 debts shown
        vals[0] = 5.0; // 6 debts
        assert_eq!(c.visible(&vals).len(), 1 + 6 * 3 + 3);
        vals[0] = 0.0;
        assert_eq!(c.visible(&vals).len(), 1 + 3 + 3);
    }

    #[test]
    fn mortgage_reference() {
        // $400k, 20% down -> $320k at 6.5%/30y = $2,022.62 P&I
        let o = run(0, 0, &[]);
        assert_eq!(value(&o, "Principal & interest"), "$2,022.62");
        // tax 1.1% of 400k / 12 = 366.67, ins 150 -> 2539.28
        assert_eq!(value(&o, "Monthly payment"), "$2,539.28");
    }

    #[test]
    fn affordability_roundtrip() {
        let o = run(0, 1, &[]);
        // 100k income -> 28% housing = 2333.33; 36% - 500 = 2500 -> housing ratio limits
        assert_eq!(value(&o, "Monthly payment (PITI)"), "$2,333.33");
        assert_eq!(value(&o, "Limited by"), "Housing ratio");
    }

    #[test]
    fn auto_loan_reference() {
        // 35000 + 6% tax (2100) + 1000 fees - 5000 down = 33100 financed, 7%/60 -> $655.42
        let o = run(1, 0, &[]);
        assert_eq!(value(&o, "Amount financed"), "$33,100.00");
        assert_eq!(value(&o, "Monthly payment"), "$655.42");
    }

    #[test]
    fn lease_reference() {
        // adj = 37500 + 995 - 2000 = 36495; residual = 23200; dep = 13295/36 = 369.31
        // mf = 0.0020833; fin = (36495 + 23200) * mf = 124.36; base 493.67; tax 6% -> 523.29
        let o = run(2, 0, &[]);
        assert_eq!(value(&o, "Depreciation portion"), "$369.31");
        assert_eq!(value(&o, "Monthly payment"), "$523.29");
    }

    #[test]
    fn tax_reference() {
        // Single, 85k - 6k pretax = 79k AGI, - 16,100 std = 62,900 taxable
        // 1240 + (50400-12400)*.12=4560 + (62900-50400)*.22=2750 -> 8550
        let o = run(8, 0, &[]);
        assert_eq!(value(&o, "Taxable income"), "$62,900.00");
        assert_eq!(value(&o, "Federal income tax"), "$8,550.00");
        assert_eq!(value(&o, "Marginal tax bracket"), "22%");
    }

    #[test]
    fn cd_reference() {
        // 10k at 4% compounded monthly for 12 months -> 10407.42, APY 4.07%
        let o = run(6, 0, &[("comp", 1.0)]);
        assert_eq!(value(&o, "Ending balance"), "$10,407.42");
        assert_eq!(value(&o, "APY"), "4.07%");
    }

    #[test]
    fn compound_growth_reference() {
        // 0 initial, $100/mo, 12% (1%/mo), 1 year -> 100 * ((1.01^12 - 1)/0.01) = 1268.25
        let o = run(
            7,
            0,
            &[
                ("initial", 0.0),
                ("monthly", 100.0),
                ("ret", 12.0),
                ("years", 1.0),
            ],
        );
        assert_eq!(value(&o, "Future value"), "$1,268");
    }

    #[test]
    fn break_even_reference() {
        let o = run(10, 0, &[]);
        assert_eq!(value(&o, "Break-even units"), "334");
    }

    #[test]
    fn tey_reference() {
        // 3.5% / (1 - .22 - .05) = 4.79%
        let o = run(8, 1, &[]);
        assert_eq!(value(&o, "Tax-equivalent yield"), "4.79%");
    }

    #[test]
    fn snowball_pays_off() {
        let o = run(3, 1, &[]);
        assert!(
            o.rows[0].kind == super::super::RowKind::Primary,
            "{:#?}",
            o.rows
        );
    }
}
