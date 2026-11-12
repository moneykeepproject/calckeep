//! Tax calculators

use super::*;

/// 2026 federal brackets (IRS Rev. Proc. 2025-32): (top of bracket, rate)
pub(super) const BRACKETS_2026: [[(f64, f64); 7]; 3] = [
    // Single
    [
        (12_400.0, 0.10),
        (50_400.0, 0.12),
        (105_700.0, 0.22),
        (201_775.0, 0.24),
        (256_225.0, 0.32),
        (640_600.0, 0.35),
        (f64::INFINITY, 0.37),
    ],
    // Married filing jointly
    [
        (24_800.0, 0.10),
        (100_800.0, 0.12),
        (211_400.0, 0.22),
        (403_550.0, 0.24),
        (512_450.0, 0.32),
        (768_700.0, 0.35),
        (f64::INFINITY, 0.37),
    ],
    // Head of household
    [
        (17_700.0, 0.10),
        (67_450.0, 0.12),
        (105_700.0, 0.22),
        (201_750.0, 0.24),
        (256_200.0, 0.32),
        (640_600.0, 0.35),
        (f64::INFINITY, 0.37),
    ],
];
pub(super) const STD_DEDUCTION_2026: [f64; 3] = [16_100.0, 32_200.0, 24_150.0];
pub(super) const SS_WAGE_BASE_2026: f64 = 184_500.0;

pub(super) fn income_tax() -> Calc {
    Calc {
        name: "Income Tax Estimate",
        blurb: "2026 federal income tax, FICA and take-home pay estimate.",
        fields: vec![
            Field::choice(
                "status",
                "Filing status",
                &["Single", "Married joint", "Head of hh."],
                0,
            ),
            Field::money("income", "Gross wages / income", 85_000.0),
            Field::money("pretax", "Pre-tax deductions", 6_000.0).help("401(k), HSA, etc."),
            Field::choice("ded", "Deduction", &["Standard", "Itemized"], 0),
            Field::money("itemized", "Itemized deductions", 0.0).show_if("ded", 1.0),
            Field::money("credits", "Tax credits", 0.0),
            Field::percent("state", "State income tax rate", 4.0, 0.0, 13.5, 0.1)
                .help("Flat estimate"),
        ],
        compute: |v| {
            let s = v.idx("status").min(2);
            let income = v.get("income");
            let agi = (income - v.get("pretax")).max(0.0);
            let ded = if v.idx("ded") == 1 {
                v.get("itemized")
            } else {
                STD_DEDUCTION_2026[s]
            };
            let taxable = (agi - ded).max(0.0);
            let (tax, marginal, parts) = bracket_tax(taxable, &BRACKETS_2026[s]);
            let fed = (tax - v.get("credits")).max(0.0);
            let medicare_threshold = if s == 1 { 250_000.0 } else { 200_000.0 };
            let fica = 0.062 * income.min(SS_WAGE_BASE_2026)
                + 0.0145 * income
                + 0.009 * (income - medicare_threshold).max(0.0);
            let state = agi * v.pct("state");
            let take_home = income - v.get("pretax") - fed - fica - state;
            let mut o = Output::default();
            o.primary("Federal income tax", money(fed));
            o.row(
                "Effective federal rate",
                pct2(if income > 0.0 {
                    fed / income * 100.0
                } else {
                    0.0
                }),
            )
            .row("Marginal tax bracket", pct(marginal * 100.0))
            .row("Taxable income", money(taxable))
            .row(
                if v.idx("ded") == 1 {
                    "Itemized deduction"
                } else {
                    "Standard deduction"
                },
                money(ded),
            )
            .row("Social Security & Medicare", money(fica))
            .row("State income tax (est.)", money(state))
            .row("Take-home pay (yearly)", money(take_home))
            .row("Take-home pay (monthly)", money(take_home / 12.0));
            if v.idx("ded") == 1 && ded < STD_DEDUCTION_2026[s] {
                o.note(format!(
                    "Your itemized deductions are below the {} standard deduction.",
                    money0(STD_DEDUCTION_2026[s])
                ));
            }
            o.note("2026 brackets (IRS Rev. Proc. 2025-32). Estimate only: ignores AMT, credit phase-outs and special deductions.");
            o.table = Some(Table {
                title: "Tax by bracket".into(),
                headers: vec!["Rate".into(), "Income taxed".into(), "Tax".into()],
                rows: parts
                    .into_iter()
                    .map(|(r, amt, t)| vec![pct(r * 100.0), money0(amt), money0(t)])
                    .collect(),
            });
            o
        },
    }
}

pub(super) fn tax_equivalent_yield() -> Calc {
    Calc {
        name: "Tax-Equivalent Yield",
        blurb: "The taxable yield a bond needs to match a tax-free municipal bond.",
        fields: vec![
            Field::percent("yield", "Tax-free yield", 3.5, 0.0, 10.0, 0.05),
            Field::choice(
                "fed",
                "Federal tax bracket",
                &["10%", "12%", "22%", "24%", "32%", "35%", "37%"],
                2,
            ),
            Field::percent("state", "State tax rate", 5.0, 0.0, 13.5, 0.1),
            Field::choice("instate", "Bond issuer", &["In-state", "Out-of-state"], 0)
                .help("In-state munis are usually state tax-free too"),
        ],
        compute: |v| {
            let fed = [0.10, 0.12, 0.22, 0.24, 0.32, 0.35, 0.37][v.idx("fed").min(6)];
            let st = v.pct("state");
            let muni_state_tax = if v.idx("instate") == 0 { 0.0 } else { st };
            let combined = fed + st;
            let tey = v.pct("yield") * (1.0 - muni_state_tax) / (1.0 - combined);
            let mut o = Output::default();
            o.primary("Tax-equivalent yield", pct2(tey * 100.0));
            o.row("Combined tax rate", pct2(combined * 100.0)).row(
                "After-tax muni yield",
                pct2(v.get("yield") * (1.0 - muni_state_tax)),
            );
            o.note(format!(
                "A taxable bond must yield more than {} to beat this tax-free yield.",
                pct2(tey * 100.0)
            ));
            o
        },
    }
}

const STATUS: [&str; 3] = ["Single", "Married joint", "Head of hh."];
/// 2026 long-term capital gains: top of 0% and 15% zones (Rev. Proc. 2025-32)
const LTCG_2026: [(f64, f64); 3] = [
    (49_450.0, 545_500.0),
    (98_900.0, 613_700.0),
    (66_200.0, 579_600.0),
];
const NIIT_THRESHOLD: [f64; 3] = [200_000.0, 250_000.0, 200_000.0];
const ADDL_MEDICARE: [f64; 3] = [200_000.0, 250_000.0, 200_000.0];

fn fed_tax(taxable: f64, status: usize) -> f64 {
    bracket_tax(taxable.max(0.0), &BRACKETS_2026[status]).0
}

pub(super) fn paycheck() -> Calc {
    Calc {
        name: "Paycheck / Take-Home Pay",
        blurb:
            "Take-home pay per paycheck after federal and state tax, FICA and pre-tax deductions.",
        fields: vec![
            Field::money("salary", "Annual salary", 75_000.0),
            Field::choice(
                "freq",
                "Pay frequency",
                &["Weekly", "Biweekly", "Twice a month", "Monthly"],
                1,
            ),
            Field::choice("status", "Filing status", &STATUS, 0),
            Field::percent("k401", "401(k) contribution", 6.0, 0.0, 50.0, 1.0),
            Field::money("health", "Pre-tax health & benefits per paycheck", 100.0),
            Field::percent("state", "State income tax rate", 4.0, 0.0, 13.5, 0.1),
        ],
        compute: |v| {
            let per = [52.0, 26.0, 24.0, 12.0][v.idx("freq").min(3)];
            let s = v.idx("status").min(2);
            let gross = v.get("salary");
            let k = (gross * v.pct("k401")).min(24_500.0);
            let health = v.get("health") * per;
            let fed = fed_tax(gross - k - health - STD_DEDUCTION_2026[s], s);
            let fica_wages = (gross - health).max(0.0);
            let ss = 0.062 * fica_wages.min(SS_WAGE_BASE_2026);
            let med = 0.0145 * fica_wages + 0.009 * (fica_wages - ADDL_MEDICARE[s]).max(0.0);
            let state = (gross - k - health).max(0.0) * v.pct("state");
            let net = gross - k - health - fed - ss - med - state;
            let mut o = Output::default();
            o.primary("Take-home per paycheck", money(net / per));
            o.row("Gross per paycheck", money(gross / per))
                .row("Federal income tax", money(fed / per))
                .row("Social Security", money(ss / per))
                .row("Medicare", money(med / per))
                .row("State income tax", money(state / per))
                .row("401(k)", money(k / per))
                .row("Health & benefits", money(health / per))
                .row("Take-home per year", money0(net));
            o.note("Estimate using 2026 brackets and the standard deduction; actual withholding depends on your W-4.");
            o
        },
    }
}

pub(super) fn self_employment() -> Calc {
    Calc {
        name: "Self-Employment & Quarterly Tax",
        blurb: "Self-employment tax, income tax and quarterly estimated payments for 1099 income.",
        fields: vec![
            Field::money("net", "Net self-employment profit", 80_000.0)
                .help("1099 income minus business expenses"),
            Field::money("wages", "W-2 wages (if any)", 0.0),
            Field::money("withheld", "Federal tax already withheld from W-2", 0.0)
                .show_if_at_least("wages", 0.01),
            Field::choice("status", "Filing status", &STATUS, 0),
            Field::choice("qbi", "20% QBI deduction", &["Apply", "Don't apply"], 0).help(
                "Qualified business income deduction; phase-outs at high incomes aren't modeled",
            ),
            Field::percent("state", "State income tax rate", 4.0, 0.0, 13.5, 0.1),
        ],
        compute: |v| {
            let s = v.idx("status").min(2);
            let net = v.get("net");
            let wages = v.get("wages");
            let se_base = (net * 0.9235).max(0.0);
            let ss = 0.124 * se_base.min((SS_WAGE_BASE_2026 - wages).max(0.0));
            let med = 0.029 * se_base
                + 0.009 * ((se_base + wages) - ADDL_MEDICARE[s].max(wages)).max(0.0);
            let se_tax = ss + med;
            let half = (ss + 0.029 * se_base) / 2.0;
            let agi = net + wages - half;
            let before_qbi = (agi - STD_DEDUCTION_2026[s]).max(0.0);
            let qbi = if v.idx("qbi") == 0 {
                (0.2 * (net - half).max(0.0)).min(0.2 * before_qbi)
            } else {
                0.0
            };
            let taxable = before_qbi - qbi;
            let income_tax = fed_tax(taxable, s);
            let state = agi.max(0.0) * v.pct("state");
            let fed_total = income_tax + se_tax;
            let quarterly = (fed_total - v.get("withheld")).max(0.0) / 4.0;
            let mut o = Output::default();
            o.primary("Quarterly federal estimate", money0(quarterly));
            o.row("Self-employment tax", money0(se_tax))
                .row("Federal income tax", money0(income_tax))
                .row("Total federal tax", money0(fed_total))
                .row("State tax (est.)", money0(state))
                .row("Quarterly state estimate", money0(state / 4.0))
                .row("QBI deduction", money0(qbi))
                .row(
                    "Effective total rate",
                    pct2((fed_total + state) / (net + wages).max(1.0) * 100.0),
                );
            o.note("Due dates: April 15, June 15, Sept 15, Jan 15. SE tax = 15.3% of 92.35% of profit (Social Security part capped at the $184,500 wage base). 2026 brackets.");
            o
        },
    }
}

pub(super) fn capital_gains() -> Calc {
    Calc {
        name: "Capital Gains Tax",
        blurb: "Federal and state tax on selling an investment, short- or long-term (2026 rates).",
        fields: vec![
            Field::money("buy", "Purchase price (cost basis)", 10_000.0),
            Field::money("sell", "Sale price", 25_000.0),
            Field::choice(
                "held",
                "Held for",
                &["More than 1 year", "1 year or less"],
                0,
            ),
            Field::choice("status", "Filing status", &STATUS, 0),
            Field::money("income", "Other taxable income", 70_000.0)
                .help("After deductions, not counting this sale"),
            Field::percent("state", "State tax rate on gains", 5.0, 0.0, 13.5, 0.1),
        ],
        compute: |v| {
            let s = v.idx("status").min(2);
            let gain = v.get("sell") - v.get("buy");
            if gain <= 0.0 {
                let mut o = Output::default();
                o.primary("Capital loss", money0(-gain));
                o.note("Losses offset other gains, plus up to $3,000 of ordinary income a year; the rest carries forward.");
                return o;
            }
            let other = v.get("income").max(0.0);
            let fed = if v.idx("held") == 0 {
                let (z0, z15) = LTCG_2026[s];
                let at0 = (z0 - other).clamp(0.0, gain);
                let at15 = ((z15 - other).max(0.0) - at0).clamp(0.0, gain - at0);
                let at20 = gain - at0 - at15;
                at15 * 0.15 + at20 * 0.20
            } else {
                fed_tax(other + gain, s) - fed_tax(other, s)
            };
            let niit = 0.038 * gain.min((other + gain - NIIT_THRESHOLD[s]).max(0.0));
            let state = gain * v.pct("state");
            let total = fed + niit + state;
            let mut o = Output::default();
            o.primary("Total tax on the gain", money0(total));
            o.row("Capital gain", money0(gain))
                .row(
                    if v.idx("held") == 0 {
                        "Federal (long-term rates)"
                    } else {
                        "Federal (ordinary rates)"
                    },
                    money0(fed),
                )
                .row("Net investment income tax (3.8%)", money0(niit))
                .row("State tax", money0(state))
                .row("Effective rate on the gain", pct2(total / gain * 100.0))
                .row("After-tax proceeds", money0(v.get("sell") - total));
            if v.idx("held") == 1 {
                let (z0, z15) = LTCG_2026[s];
                let at0 = (z0 - other).clamp(0.0, gain);
                let at15 = ((z15 - other).max(0.0) - at0).clamp(0.0, gain - at0);
                let lt = at15 * 0.15 + (gain - at0 - at15) * 0.20;
                o.note(format!(
                    "Holding more than a year would cut the federal tax to about {}.",
                    money0(lt)
                ));
            }
            o.note("2026 long-term brackets (Rev. Proc. 2025-32). The 3.8% NIIT uses income as an estimate of MAGI.");
            o
        },
    }
}
