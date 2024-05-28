use super::super::*;
use ark_bn254::Fr;
use num_bigint::BigUint;
use std::collections::HashMap;

fn f2usize(f: Fr) -> u64 {
    let bigint: BigUint = f.into();
    let bytes = bigint.to_bytes_be();
    let mut res = 0;
    for i in 0..bytes.len() {
        res = res * 256 + bytes[i] as u64;
    }
    res
}

pub fn to_tx(raw_tx: (&String, HashMap<String, Fr>)) -> Result<Tx, String> {
    let op_type = raw_tx.0;
    let params = raw_tx.1;
    match op_type.as_str() {
        "noop" => Ok(Tx::TxNoop(to_noop(params)?)),
        "register" => Ok(Tx::TxRegister(to_register(params)?)),
        "deposit" => Ok(Tx::TxDeposit(to_deposit(params)?)),
        "forced_withdraw" => Ok(Tx::TxForcedWithdraw(to_forced_withdraw(params)?)),
        "transfer" => Ok(Tx::TxTransfer(to_transfer(params)?)),
        "withdraw" => Ok(Tx::TxWithdraw(to_withdraw(params)?)),
        "auction_lend" => Ok(Tx::TxAucLend(to_auc_lend(params)?)),
        "auction_borrow" => Ok(Tx::TxAucBorrow(to_auc_borrow(params)?)),
        "auction_start" => Ok(Tx::TxAucStart(to_auc_start(params)?)),
        "auction_match" => Ok(Tx::TxAucMatch(to_auc_match(params)?)),
        "auction_end" => Ok(Tx::TxAucEnd(to_auc_end(params)?)),
        "second_limit_order" => Ok(Tx::TxSecLimitOrder(to_sec_limit_order(params)?)),
        "second_limit_start" => Ok(Tx::TxSecLimitStart(to_sec_limit_start(params)?)),
        "second_limit_exchange" => Ok(Tx::TxSecLimitExchange(to_sec_limit_exchange(params)?)),
        "second_limit_end" => Ok(Tx::TxSecLimitEnd(to_sec_limit_end(params)?)),
        "second_market_order" => Ok(Tx::TxSecMarketOrder(to_sec_market_order(params)?)),
        "second_market_exchange" => Ok(Tx::TxSecMarketExchange(to_sec_market_exchange(params)?)),
        "second_market_end" => Ok(Tx::TxSecMarketEnd(to_sec_market_end(params)?)),
        "admin_cancel" => Ok(Tx::TxAdminCancel(to_admin_cancel(params)?)),
        "user_cancel" => Ok(Tx::TxUserCancel(to_user_cancel(params)?)),
        "increase_epoch" => Ok(Tx::TxIncreaseEpoch(to_increase_epoch(params)?)),
        "create_bond_token" => Ok(Tx::TxCreateTsbBondToken(to_create_tsb_bond_token(params)?)),
        "redeem" => Ok(Tx::TxRedeem(to_redeem(params)?)),
        "withdraw_fee" => Ok(Tx::TxWithdrawFee(to_withdraw_fee(params)?)),
        "evacuation" => Ok(Tx::TxEvacuation(to_evacuation(params)?)),
        "set_admin_ts_addr" => Ok(Tx::TxSetAdminTsAddr(to_set_admin_ts_add(params)?)),
        _ => Err(format!("invalid op: {}", op_type).to_string()),
    }
}

pub fn to_noop(_: HashMap<String, Fr>) -> Result<TxNoop, String> {
    Ok(TxNoop {})
}
pub fn to_register(raw_tx: HashMap<String, Fr>) -> Result<TxRegister, String> {
    let account_id = f2usize(*raw_tx.get("account_id").ok_or("invalid op")?);
    let hashed_pub_key = *raw_tx.get("hashed_pub_key").ok_or("invalid op")?;
    Ok(TxRegister {
        account_id,
        hashed_pub_key,
    })
}
pub fn to_deposit(raw_tx: HashMap<String, Fr>) -> Result<TxDeposit, String> {
    let account_id = f2usize(*raw_tx.get("account_id").ok_or("invalid op")?);
    let deposit_token_id = f2usize(*raw_tx.get("deposit_token_id").ok_or("invalid op")?);
    let deposit_amt = *raw_tx.get("deposit_amt").ok_or("invalid op")?;
    Ok(TxDeposit {
        account_id,
        deposit_token_id,
        deposit_amt,
    })
}
pub fn to_forced_withdraw(raw_tx: HashMap<String, Fr>) -> Result<TxForcedWithdraw, String> {
    let account_id = f2usize(*raw_tx.get("account_id").ok_or("invalid op")?);
    let withdraw_token_id = f2usize(*raw_tx.get("withdraw_token_id").ok_or("invalid op")?);
    let withdraw_amt = *raw_tx.get("withdraw_amt").ok_or("invalid op")?;
    Ok(TxForcedWithdraw {
        account_id,
        withdraw_token_id,
        withdraw_amt,
    })
}
pub fn to_transfer(raw_tx: HashMap<String, Fr>) -> Result<TxTransfer, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let transfer_token_id = f2usize(*raw_tx.get("transfer_token_id").ok_or("invalid op")?);
    let transfer_amt = *raw_tx.get("transfer_amt").ok_or("invalid op")?;
    let receiver_id = f2usize(*raw_tx.get("receiver_id").ok_or("invalid op")?);
    Ok(TxTransfer {
        sender_id,
        transfer_token_id,
        transfer_amt,
        receiver_id,
    })
}
pub fn to_withdraw(raw_tx: HashMap<String, Fr>) -> Result<TxWithdraw, String> {
    let account_id = f2usize(*raw_tx.get("account_id").ok_or("invalid op")?);
    let withdraw_token_id = f2usize(*raw_tx.get("withdraw_token_id").ok_or("invalid op")?);
    let withdraw_amt = *raw_tx.get("withdraw_amt").ok_or("invalid op")?;
    let tx_fee_token_id = f2usize(*raw_tx.get("tx_fee_token_id").ok_or("invalid op")?);
    let tx_fee_amt = *raw_tx.get("tx_fee_amt").ok_or("invalid op")?;
    Ok(TxWithdraw {
        account_id,
        withdraw_token_id,
        withdraw_amt,
        tx_fee_token_id,
        tx_fee_amt,
    })
}
pub fn to_auc_lend(raw_tx: HashMap<String, Fr>) -> Result<TxAucLend, String> {
    let lender_id = f2usize(*raw_tx.get("lender_id").ok_or("invalid op")?);
    let lending_token_id = f2usize(*raw_tx.get("lending_token_id").ok_or("invalid op")?);
    let lending_amt = *raw_tx.get("lending_amt").ok_or("invalid op")?;
    let fee_rate = *raw_tx.get("fee_rate").ok_or("invalid op")?;
    let default_matched_interest_rate = *raw_tx
        .get("default_matched_interest_rate")
        .ok_or("invalid op")?;
    let maturity_time = *raw_tx.get("maturity_time").ok_or("invalid op")?;
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    let primary_lend_min_fee_amt = *raw_tx.get("primary_lend_min_fee_amt").ok_or("invalid op")?;
    Ok(TxAucLend {
        lender_id,
        lending_token_id,
        lending_amt,
        fee_rate,
        default_matched_interest_rate,
        maturity_time,
        matched_time,
        primary_lend_min_fee_amt,
    })
}
pub fn to_auc_borrow(raw_tx: HashMap<String, Fr>) -> Result<TxAucBorrow, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let collateral_token_id = f2usize(*raw_tx.get("collateral_token_id").ok_or("invalid op")?);
    let collateral_amt = *raw_tx.get("collateral_amt").ok_or("invalid op")?;
    let fee_rate = *raw_tx.get("fee_rate").ok_or("invalid op")?;
    let borrowing_amt = *raw_tx.get("borrowing_amt").ok_or("invalid op")?;
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    let primary_borrow_min_fee_amt = *raw_tx.get("primary_borrow_min_fee_amt").ok_or("invalid op")?;
    Ok(TxAucBorrow {
        sender_id,
        collateral_token_id,
        collateral_amt,
        fee_rate,
        borrowing_amt,
        matched_time,
        primary_borrow_min_fee_amt,
    })
}
pub fn to_auc_start(raw_tx: HashMap<String, Fr>) -> Result<TxAucStart, String> {
    let borrower_tx_offset = f2usize(*raw_tx.get("borrower_tx_offset").ok_or("invalid op")?);
    let ori_matched_interest = *raw_tx.get("ori_matched_interest").ok_or("invalid op")?;
    Ok(TxAucStart {
        borrower_tx_offset,
        ori_matched_interest,
    })
}
pub fn to_auc_match(raw_tx: HashMap<String, Fr>) -> Result<TxAucMatch, String> {
    let lender_tx_offset = f2usize(*raw_tx.get("lender_tx_offset").ok_or("invalid op")?);
    Ok(TxAucMatch { lender_tx_offset })
}
pub fn to_auc_end(raw_tx: HashMap<String, Fr>) -> Result<TxAucEnd, String> {
    let borrow_account = *raw_tx.get("borrow_account").ok_or("invalid op")?;
    let collateral_token_id = f2usize(*raw_tx.get("collateral_token_id").ok_or("invalid op")?);
    let collateral_amt = *raw_tx.get("collateral_amt").ok_or("invalid op")?;
    let debt_token_id = f2usize(*raw_tx.get("debt_token_id").ok_or("invalid op")?);
    let debt_amt = *raw_tx.get("debt_amt").ok_or("invalid op")?;
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    let maturity = *raw_tx.get("maturity").ok_or("invalid op")?;
    Ok(TxAucEnd {
        borrow_account,
        collateral_token_id,
        collateral_amt,
        debt_token_id,
        debt_amt,
        matched_time,
        maturity
    })
}
pub fn to_sec_limit_order(raw_tx: HashMap<String, Fr>) -> Result<TxSecLimitOrder, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let sell_token_id = f2usize(*raw_tx.get("sell_token_id").ok_or("invalid op")?);
    let sell_amt = *raw_tx.get("sell_amt").ok_or("invalid op")?;
    let fee0 = *raw_tx.get("fee0").ok_or("invalid op")?;
    let fee1 = *raw_tx.get("fee1").ok_or("invalid op")?;
    let buy_token_id = f2usize(*raw_tx.get("buy_token_id").ok_or("invalid op")?);
    let buy_amt = *raw_tx.get("buy_amt").ok_or("invalid op")?;
    let expired_time = *raw_tx.get("expired_time").ok_or("invalid op")?;
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    let secondary_taker_min_fee_amt = *raw_tx
        .get("secondary_taker_min_fee_amt")
        .ok_or("invalid op")?;
    let secondary_maker_min_fee_amt = *raw_tx
        .get("secondary_maker_min_fee_amt")
        .ok_or("invalid op")?;
    Ok(TxSecLimitOrder {
        sender_id,
        sell_token_id,
        sell_amt,
        fee0,
        fee1,
        buy_token_id,
        buy_amt,
        expired_time,
        matched_time,
        secondary_taker_min_fee_amt,
        secondary_maker_min_fee_amt,
    })
}
pub fn to_sec_limit_start(raw_tx: HashMap<String, Fr>) -> Result<TxSecLimitStart, String> {
    let taker_tx_offset = f2usize(*raw_tx.get("taker_tx_offset").ok_or("invalid op")?);
    Ok(TxSecLimitStart { taker_tx_offset })
}
pub fn to_sec_limit_exchange(raw_tx: HashMap<String, Fr>) -> Result<TxSecLimitExchange, String> {
    let maker_tx_offset = f2usize(*raw_tx.get("maker_tx_offset").ok_or("invalid op")?);
    Ok(TxSecLimitExchange { maker_tx_offset })
}
pub fn to_sec_limit_end(raw_tx: HashMap<String, Fr>) -> Result<TxSecLimitEnd, String> {
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    Ok(TxSecLimitEnd { matched_time })
}
pub fn to_sec_market_order(raw_tx: HashMap<String, Fr>) -> Result<TxSecMarketOrder, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let sell_token_id = f2usize(*raw_tx.get("sell_token_id").ok_or("invalid op")?);
    let sell_amt = *raw_tx.get("sell_amt").ok_or("invalid op")?;
    let fee0 = *raw_tx.get("fee0").ok_or("invalid op")?;
    let buy_token_id = f2usize(*raw_tx.get("buy_token_id").ok_or("invalid op")?);
    let buy_amt = *raw_tx.get("buy_amt").ok_or("invalid op")?;
    let expired_time = *raw_tx.get("expired_time").ok_or("invalid op")?;
    let secondary_taker_min_fee_amt = *raw_tx
        .get("secondary_taker_min_fee_amt")
        .ok_or("invalid op")?;
    Ok(TxSecMarketOrder {
        sender_id,
        sell_token_id,
        sell_amt,
        fee0,
        buy_token_id,
        buy_amt,
        expired_time,
        secondary_taker_min_fee_amt,
    })
}
pub fn to_sec_market_exchange(raw_tx: HashMap<String, Fr>) -> Result<TxSecMarketExchange, String> {
    let maker_tx_offset = f2usize(*raw_tx.get("maker_tx_offset").ok_or("invalid op")?);
    Ok(TxSecMarketExchange { maker_tx_offset })
}
pub fn to_sec_market_end(raw_tx: HashMap<String, Fr>) -> Result<TxSecMarketEnd, String> {
    let matched_time = *raw_tx.get("matched_time").ok_or("invalid op")?;
    Ok(TxSecMarketEnd { matched_time })
}
pub fn to_admin_cancel(raw_tx: HashMap<String, Fr>) -> Result<TxAdminCancel, String> {
    let tx_id = f2usize(*raw_tx.get("tx_id").ok_or("invalid op")?);
    Ok(TxAdminCancel { tx_id })
}
pub fn to_user_cancel(raw_tx: HashMap<String, Fr>) -> Result<TxUserCancel, String> {
    let tx_id = f2usize(*raw_tx.get("tx_id").ok_or("invalid op")?);
    let tx_fee_token_id = f2usize(*raw_tx.get("tx_fee_token_id").ok_or("invalid op")?);
    let tx_fee_amt = *raw_tx.get("tx_fee_amt").ok_or("invalid op")?;
    Ok(TxUserCancel {
        tx_id,
        tx_fee_token_id,
        tx_fee_amt,
    })
}
pub fn to_increase_epoch(_: HashMap<String, Fr>) -> Result<TxIncreaseEpoch, String> {
    Ok(TxIncreaseEpoch {})
}
pub fn to_create_tsb_bond_token(
    raw_tx: HashMap<String, Fr>,
) -> Result<TxCreateTsbBondToken, String> {
    let maturity = *raw_tx.get("maturity").ok_or("invalid op")?;
    let base_token_id = f2usize(*raw_tx.get("base_token_id").ok_or("invalid op")?);
    let bond_token_id = f2usize(*raw_tx.get("bond_token_id").ok_or("invalid op")?);
    Ok(TxCreateTsbBondToken {
        maturity,
        base_token_id,
        bond_token_id,
    })
}
pub fn to_redeem(raw_tx: HashMap<String, Fr>) -> Result<TxRedeem, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let token_id = f2usize(*raw_tx.get("token_id").ok_or("invalid op")?);
    let amount = *raw_tx.get("amount").ok_or("invalid op")?;
    Ok(TxRedeem {
        sender_id,
        token_id,
        amount,
    })
}
pub fn to_withdraw_fee(raw_tx: HashMap<String, Fr>) -> Result<TxWithdrawFee, String> {
    let token_id = f2usize(*raw_tx.get("token_id").ok_or("invalid op")?);
    let amount = *raw_tx.get("amount").ok_or("invalid op")?;
    Ok(TxWithdrawFee { token_id, amount })
}
pub fn to_evacuation(raw_tx: HashMap<String, Fr>) -> Result<TxEvacuation, String> {
    let sender_id = f2usize(*raw_tx.get("sender_id").ok_or("invalid op")?);
    let token_id = f2usize(*raw_tx.get("token_id").ok_or("invalid op")?);
    let amount = *raw_tx.get("amount").ok_or("invalid op")?;
    Ok(TxEvacuation {
        sender_id,
        token_id,
        amount,
    })
}
pub fn to_set_admin_ts_add(_: HashMap<String, Fr>) -> Result<TxSetAdminTsAddr, String> {
    Ok(TxSetAdminTsAddr {})
}
