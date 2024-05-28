use super::{super::Tx, _1f, _1fixed, _365f};
use ark_bn254::Fr;
use num_bigint::BigUint;
use ts_tx::Tx as RawTx;

// $$debtAmt := principal * (PIR * (days - 1) + one * (365 - (days - 1))) / (365 * one)$$
pub fn calc_debt_amt(pir: Fr, principal: Fr, days: Fr) -> Fr {
    let numerator: BigUint =
        (principal * (pir * (days - _1f()) + _1fixed() * (_365f() - (days - _1f())))).into();
    let denominator: BigUint = (_365f() * _1fixed()).into();
    (numerator / denominator).into()
}

// $$fee := matchedBorrowingAmt * |interest| * feeRate * (days - 1) / (365 * one * one)$$
pub fn calc_fee(fee_rate: Fr, matched_borrow_amt: Fr, matched_pir: Fr, days: Fr) -> Fr {
    let abs_interest = if matched_pir > _1fixed() {
        matched_pir - _1fixed()
    } else {
        _1fixed() - matched_pir
    };
    let numerator: BigUint = (matched_borrow_amt * abs_interest * fee_rate * (days - _1f())).into();
    let denominator: BigUint = (_365f() * _1fixed() * _1fixed()).into();
    (numerator / denominator).into()
}

pub fn mechanism(
    borrower: &mut Tx,
    lender: &mut Tx,
    days: Fr,
    matched_pir: Fr,
) -> Result<(), String> {
    match (borrower.raw_tx, lender.raw_tx) {
        (RawTx::TxAucBorrow(raw_borrower), RawTx::TxAucLend(raw_lender)) => {
            let remain_lend_amt = raw_lender.lending_amt - lender.cum_deducted_amt;
            let remain_borrow_amt = raw_borrower.borrowing_amt - borrower.cum_target_amt;
            let matched_amt = if remain_lend_amt > remain_borrow_amt {
                remain_borrow_amt
            } else {
                remain_lend_amt
            };
            let matched_debt_amt = calc_debt_amt(matched_pir, matched_amt, days);
            lender.cum_deducted_amt += matched_amt;
            lender.cum_target_amt += matched_debt_amt;
            borrower.cum_target_amt += matched_amt;
            borrower.cum_deducted_amt = {
                let numerator: BigUint = (raw_borrower.collateral_amt
                    * (raw_borrower.borrowing_amt - borrower.cum_target_amt))
                    .into();
                let denominator: BigUint = (raw_borrower.borrowing_amt).into();
                let tmp: Fr = (numerator / denominator).into();
                raw_borrower.collateral_amt - tmp
            };
            Ok(())
        }
        _ => Err("invalid tx type".to_string()),
    }
}
