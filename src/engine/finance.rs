//! Core time-value-of-money helpers shared by the calculators.

/// Payment per period for a loan of `pv` over `n` periods at periodic rate `r`.
pub fn pmt(r: f64, n: f64, pv: f64) -> f64 {
    if n <= 0.0 {
        return 0.0;
    }
    if r.abs() < 1e-12 {
        pv / n
    } else {
        pv * r / (1.0 - (1.0 + r).powf(-n))
    }
}

/// Present value of `n` payments of `p` at periodic rate `r`.
pub fn pv_of_payments(r: f64, n: f64, p: f64) -> f64 {
    if r.abs() < 1e-12 {
        p * n
    } else {
        p * (1.0 - (1.0 + r).powf(-n)) / r
    }
}

/// Remaining balance after `k` payments.
pub fn balance_after(pv: f64, r: f64, p: f64, k: f64) -> f64 {
    if r.abs() < 1e-12 {
        (pv - p * k).max(0.0)
    } else {
        let g = (1.0 + r).powf(k);
        (pv * g - p * (g - 1.0) / r).max(0.0)
    }
}

/// One summarized row of an amortization schedule.
#[derive(Clone, Debug, Default)]
pub struct AmortRow {
    /// 1-based year (or period index when grouping by period)
    pub index: u32,
    pub principal: f64,
    pub interest: f64,
    pub balance: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Amortization {
    pub periods: u32,
    pub total_interest: f64,
    pub total_paid: f64,
    pub paid_off: bool,
    /// Grouped rows (`group` periods per row; 12 for yearly summary of a monthly loan).
    pub rows: Vec<AmortRow>,
}

/// Simulate a loan period by period with an optional extra payment each period.
pub fn amortize(
    pv: f64,
    r: f64,
    payment: f64,
    extra: f64,
    max_periods: u32,
    group: u32,
) -> Amortization {
    let mut a = Amortization::default();
    let mut bal = pv;
    let mut acc = AmortRow::default();
    let mut k = 0;
    while bal > 0.005 && k < max_periods {
        k += 1;
        let int = bal * r;
        let pay = (payment + extra).min(bal + int);
        let princ = pay - int;
        if princ <= 0.0 {
            // payment doesn't cover interest: never pays off
            a.periods = k;
            a.paid_off = false;
            return a;
        }
        bal -= princ;
        if bal < 0.005 {
            bal = 0.0;
        }
        a.total_interest += int;
        a.total_paid += pay;
        acc.principal += princ;
        acc.interest += int;
        if k % group == 0 || bal == 0.0 {
            acc.index = (k - 1) / group + 1;
            acc.balance = bal;
            a.rows.push(std::mem::take(&mut acc));
        }
    }
    a.periods = k;
    a.paid_off = bal <= 0.005;
    a
}

/// Months needed to pay `balance` at monthly rate `r` with `payment`; `None` if never.
pub fn months_to_payoff(balance: f64, r: f64, payment: f64) -> Option<u32> {
    let a = amortize(balance, r, payment, 0.0, 1200, 1200);
    a.paid_off.then_some(a.periods)
}

/// U.S. federal tax on `taxable` income given (upper bound, rate) brackets.
/// Returns (tax, marginal rate, per-bracket amounts).
pub fn bracket_tax(taxable: f64, brackets: &[(f64, f64)]) -> (f64, f64, Vec<(f64, f64, f64)>) {
    let mut tax = 0.0;
    let mut lower = 0.0;
    let mut marginal = brackets.first().map(|b| b.1).unwrap_or(0.0);
    let mut parts = vec![];
    for &(upper, rate) in brackets {
        if taxable <= lower {
            break;
        }
        let amt = taxable.min(upper) - lower;
        tax += amt * rate;
        parts.push((rate, amt, amt * rate));
        marginal = rate;
        lower = upper;
    }
    (tax, marginal, parts)
}

/// Find x in [lo, hi] where f(x) = 0 by bisection. Needs a sign change.
pub fn solve(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> Option<f64> {
    let (mut flo, fhi) = (f(lo), f(hi));
    if !flo.is_finite() || !fhi.is_finite() || flo * fhi > 0.0 {
        return None;
    }
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        let fm = f(mid);
        if fm == 0.0 || (hi - lo).abs() < 1e-12 {
            return Some(mid);
        }
        if (fm < 0.0) == (flo < 0.0) {
            lo = mid;
            flo = fm;
        } else {
            hi = mid;
        }
    }
    Some((lo + hi) / 2.0)
}

/// Periodic rate at which `n` payments of `p` repay `pv` (e.g. the true APR of a
/// loan whose fees reduce the cash you receive).
pub fn rate_for(pv: f64, p: f64, n: f64) -> Option<f64> {
    if pv <= 0.0 || p * n < pv {
        return None;
    }
    solve(|r| pv_of_payments(r, n, p) - pv, 0.0, 1.0)
}

/// Net present value of yearly cash flows (index 0 = today).
pub fn npv(rate: f64, flows: &[f64]) -> f64 {
    flows
        .iter()
        .enumerate()
        .map(|(t, c)| c / (1.0 + rate).powi(t as i32))
        .sum()
}

/// Internal rate of return of yearly cash flows.
pub fn irr(flows: &[f64]) -> Option<f64> {
    solve(|r| npv(r, flows), -0.99, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    #[test]
    fn mortgage_payment_matches_reference() {
        // $320,000 at 6.5% for 30 years -> $2,022.62/mo (standard reference value)
        close(pmt(0.065 / 12.0, 360.0, 320_000.0), 2022.62, 0.01);
        // $200,000 at 6% for 30 years -> $1,199.10
        close(pmt(0.005, 360.0, 200_000.0), 1199.10, 0.01);
        close(pmt(0.0, 60.0, 30_000.0), 500.0, 1e-9);
    }

    #[test]
    fn amortization_totals() {
        let p = pmt(0.005, 360.0, 200_000.0);
        let a = amortize(200_000.0, 0.005, p, 0.0, 400, 12);
        assert!(a.paid_off);
        assert_eq!(a.periods, 360);
        assert_eq!(a.rows.len(), 30);
        close(a.total_interest, 231_676.38, 1.0);
        close(balance_after(200_000.0, 0.005, p, 360.0), 0.0, 0.01);
    }

    #[test]
    fn pv_roundtrip() {
        let r = 0.07 / 12.0;
        let p = pmt(r, 60.0, 25_000.0);
        close(pv_of_payments(r, 60.0, p), 25_000.0, 1e-6);
    }

    #[test]
    fn solver_and_irr() {
        // $10,000 loan, 36 payments of $332.14 -> 12% APR
        let r = rate_for(10_000.0, 332.14, 36.0).unwrap();
        close(r * 12.0, 0.12, 1e-4);
        // -1000 then +1100 in a year -> 10%
        close(irr(&[-1000.0, 1100.0]).unwrap(), 0.10, 1e-9);
        close(npv(0.1, &[-1000.0, 1100.0]), 0.0, 1e-9);
    }

    #[test]
    fn payoff_months() {
        // $5,000 at 18% paying $200/mo: about 32 months
        assert_eq!(months_to_payoff(5_000.0, 0.015, 200.0), Some(32));
        assert_eq!(months_to_payoff(5_000.0, 0.015, 75.0), None);
    }
}
