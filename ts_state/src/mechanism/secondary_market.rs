use super::{super::Tx, _1fixed, _365f};
use ark_bn254::Fr;
use num_bigint::BigUint;
use ts_tx::Tx as RawTx;

// $$newBQ := 365 * MQ * priceBQ / (days * (priceMQ - priceBQ) + 365 * priceBQ)$$
pub fn calc_new_bq(mq: Fr, price_mq: Fr, price_bq: Fr, days: Fr) -> Fr {
    let numerator: BigUint = (_365f() * mq * price_bq).into();
    let denominator: BigUint = (days * (price_mq - price_bq) + _365f() * price_bq).into();
    (numerator / denominator).into()
}

// $$supMQ := (avlBQ * days * priceMQ + (365 - days) * priceBQ * avlBQ) / (365 * priceBQ)$$
pub fn calc_sup_mq(avl_bq: Fr, price_mq: Fr, price_bq: Fr, days: Fr) -> Fr {
    let numerator: BigUint =
        (avl_bq * days * price_mq + (_365f() - days) * price_bq * avl_bq).into();
    let denominator: BigUint = (_365f() * price_bq).into();
    (numerator / denominator).into()
}

// $$fee := MQ * feeRate * days / (365 * one)$$
pub fn calc_fee(fee_rate: Fr, mq: Fr, days: Fr) -> Fr {
    let numerator: BigUint = (mq * fee_rate * days).into();
    let denominator: BigUint = (_365f() * _1fixed()).into();
    (numerator / denominator).into()
}

pub fn mechanism(taker: &mut Tx, maker: &mut Tx, days: Fr, maker_side: bool) -> Result<(), String> {
    match (taker.raw_tx, maker.raw_tx) {
        (RawTx::TxSecLimitOrder(raw_taker), RawTx::TxSecLimitOrder(raw_maker)) => {
            let (taker_mq, maker_mq, maker_bq) = if maker_side {
                (raw_taker.buy_amt, raw_maker.sell_amt, raw_maker.buy_amt)
            } else {
                (raw_taker.sell_amt, raw_maker.buy_amt, raw_maker.sell_amt)
            };
            let (remain_taker_mq, remain_maker_mq) = if maker_side {
                (
                    taker_mq - taker.cum_target_amt,
                    maker_mq - maker.cum_deducted_amt,
                )
            } else {
                (
                    taker_mq - taker.cum_deducted_amt,
                    maker_mq - maker.cum_target_amt,
                )
            };
            let matched_mq = if remain_taker_mq > remain_maker_mq {
                remain_maker_mq
            } else {
                remain_taker_mq
            };
            let matched_bq = calc_new_bq(matched_mq, maker_mq, maker_bq, days);

            let (matched_maker_sell_amt, matched_maker_buy_amt) = if maker_side {
                (matched_mq, matched_bq)
            } else {
                (matched_bq, matched_mq)
            };

            maker.cum_deducted_amt += matched_maker_sell_amt;
            maker.cum_target_amt += matched_maker_buy_amt;
            taker.cum_deducted_amt += matched_maker_buy_amt;
            taker.cum_target_amt += matched_maker_sell_amt;

            Ok(())
        }
        (RawTx::TxSecMarketOrder(raw_taker), RawTx::TxSecLimitOrder(raw_maker)) => {
            let (taker_mq, maker_mq, maker_bq) = if maker_side {
                (raw_taker.buy_amt, raw_maker.sell_amt, raw_maker.buy_amt)
            } else {
                (raw_taker.sell_amt, raw_maker.buy_amt, raw_maker.sell_amt)
            };
            let (remain_taker_mq, remain_maker_mq) = if maker_side {
                (
                    taker_mq - taker.cum_target_amt,
                    maker_mq - maker.cum_deducted_amt,
                )
            } else {
                (
                    taker_mq - taker.cum_deducted_amt,
                    maker_mq - maker.cum_target_amt,
                )
            };
            let mut matched_mq = if remain_taker_mq > remain_maker_mq {
                remain_maker_mq
            } else {
                remain_taker_mq
            };
            let mut matched_bq = calc_new_bq(matched_mq, maker_mq, maker_bq, days);

            if maker_side {
                let taker_avl_bq = raw_taker.sell_amt;
                let remain_taker_bq = taker_avl_bq - taker.cum_deducted_amt;
                if remain_taker_bq < matched_bq {
                    matched_bq = remain_taker_bq;
                    matched_mq = calc_sup_mq(matched_bq, maker_mq, maker_bq, days);
                }
            }

            let (matched_maker_sell_amt, matched_maker_buy_amt) = if maker_side {
                (matched_mq, matched_bq)
            } else {
                (matched_bq, matched_mq)
            };

            maker.cum_deducted_amt += matched_maker_sell_amt;
            maker.cum_target_amt += matched_maker_buy_amt;
            taker.cum_deducted_amt += matched_maker_buy_amt;
            taker.cum_target_amt += matched_maker_sell_amt;

            Ok(())
        }
        _ => Err("invalid tx type".to_string()),
    }
}
