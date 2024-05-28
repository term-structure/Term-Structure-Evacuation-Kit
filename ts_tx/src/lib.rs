pub mod parser;
use ark_bn254::Fr;
use std::fmt::Debug;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum Tx {
    TxNoop(TxNoop),
    TxRegister(TxRegister),
    TxDeposit(TxDeposit),
    TxForcedWithdraw(TxForcedWithdraw),
    TxTransfer(TxTransfer),
    TxWithdraw(TxWithdraw),
    TxAucLend(TxAucLend),
    TxAucBorrow(TxAucBorrow),
    TxAucStart(TxAucStart),
    TxAucMatch(TxAucMatch),
    TxAucEnd(TxAucEnd),
    TxSecLimitOrder(TxSecLimitOrder),
    TxSecLimitStart(TxSecLimitStart),
    TxSecLimitExchange(TxSecLimitExchange),
    TxSecLimitEnd(TxSecLimitEnd),
    TxSecMarketOrder(TxSecMarketOrder),
    TxSecMarketExchange(TxSecMarketExchange),
    TxSecMarketEnd(TxSecMarketEnd),
    TxAdminCancel(TxAdminCancel),
    TxUserCancel(TxUserCancel),
    TxIncreaseEpoch(TxIncreaseEpoch),
    TxCreateTsbBondToken(TxCreateTsbBondToken),
    TxRedeem(TxRedeem),
    TxWithdrawFee(TxWithdrawFee),
    TxEvacuation(TxEvacuation),
    TxSetAdminTsAddr(TxSetAdminTsAddr),
}
impl Default for Tx {
    fn default() -> Self {
        Tx::TxNoop(TxNoop {})
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TxNoop {}
#[derive(Clone, Copy, Debug)]
pub struct TxRegister {
    pub account_id: u64,
    pub hashed_pub_key: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxDeposit {
    pub account_id: u64,
    pub deposit_token_id: u64,
    pub deposit_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxForcedWithdraw {
    pub account_id: u64,
    pub withdraw_token_id: u64,
    pub withdraw_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxTransfer {
    pub sender_id: u64,
    pub transfer_token_id: u64,
    pub transfer_amt: Fr,
    pub receiver_id: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxWithdraw {
    pub account_id: u64,
    pub withdraw_token_id: u64,
    pub withdraw_amt: Fr,
    pub tx_fee_token_id: u64,
    pub tx_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAucLend {
    pub lender_id: u64,
    pub lending_token_id: u64,
    pub lending_amt: Fr,
    pub fee_rate: Fr,
    pub default_matched_interest_rate: Fr,
    pub maturity_time: Fr,
    pub matched_time: Fr,
    pub primary_lend_min_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAucBorrow {
    pub sender_id: u64,
    pub collateral_token_id: u64,
    pub collateral_amt: Fr,
    pub fee_rate: Fr,
    pub borrowing_amt: Fr,
    pub matched_time: Fr,
    pub primary_borrow_min_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAucStart {
    pub borrower_tx_offset: u64,
    pub ori_matched_interest: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAucMatch {
    pub lender_tx_offset: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAucEnd {
    pub borrow_account: Fr,
    pub collateral_token_id: u64,
    pub collateral_amt: Fr,
    pub debt_token_id: u64,
    pub debt_amt: Fr,
    pub matched_time: Fr,
    pub maturity: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecLimitOrder {
    pub sender_id: u64,
    pub sell_token_id: u64,
    pub sell_amt: Fr,
    pub fee0: Fr,
    pub fee1: Fr,
    pub buy_token_id: u64,
    pub buy_amt: Fr,
    pub expired_time: Fr,
    pub matched_time: Fr,
    pub secondary_taker_min_fee_amt: Fr,
    pub secondary_maker_min_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecLimitStart {
    pub taker_tx_offset: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecLimitExchange {
    pub maker_tx_offset: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecLimitEnd {
    pub matched_time: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecMarketOrder {
    pub sender_id: u64,
    pub sell_token_id: u64,
    pub sell_amt: Fr,
    pub fee0: Fr,
    pub buy_token_id: u64,
    pub buy_amt: Fr,
    pub expired_time: Fr,
    pub secondary_taker_min_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecMarketExchange {
    pub maker_tx_offset: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSecMarketEnd {
    pub matched_time: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxAdminCancel {
    pub tx_id: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxUserCancel {
    pub tx_id: u64,
    pub tx_fee_token_id: u64,
    pub tx_fee_amt: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxIncreaseEpoch {}
#[derive(Clone, Copy, Debug)]
pub struct TxCreateTsbBondToken {
    pub maturity: Fr,
    pub base_token_id: u64,
    pub bond_token_id: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct TxRedeem {
    pub sender_id: u64,
    pub token_id: u64,
    pub amount: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxWithdrawFee {
    pub token_id: u64,
    pub amount: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxEvacuation {
    pub sender_id: u64,
    pub token_id: u64,
    pub amount: Fr,
}
#[derive(Clone, Copy, Debug)]
pub struct TxSetAdminTsAddr {}
