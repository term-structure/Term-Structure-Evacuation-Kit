mod account;
pub mod constants;
mod mechanism;
mod token;
mod tsb_info;
mod tx;

use self::mechanism::{calc_days, primary_market};
pub use self::{
    account::{Account, AccountTree},
    mechanism::secondary_market,
    token::{Token, TokenTree},
    tsb_info::TSBInfo,
    tx::Tx,
};
use ark_bn254::Fr;
use num_traits::Zero;
use ts_merkle_tree::{MerkleTree, MerkleTreeWithLeaves};
use ts_poseidon::poseidon;
use ts_tx::Tx as RawTx;

pub trait Value: Sized {
    fn get(&self) -> Result<Fr, String>;
    fn set(&mut self, value: &Fr) -> Result<(), String>;
}
pub trait Array<Elem>: Sized {
    fn get(&self, index: usize) -> Result<Elem, String>;
    fn set(&mut self, index: usize, elem: &Elem) -> Result<(), String>;
}
pub struct State<
    TsRoot: Value,
    AccountTreeNodes: Array<Option<Fr>>,
    Accounts: Array<Account<TokenTreeNodes, Tokens>>,
    TokenTreeNodes: Array<Option<Fr>>,
    Tokens: Array<Token>,
    TSBInfos: Array<tsb_info::TSBInfo>,
    Txs: Array<Tx>,
> {
    pub ts_root: TsRoot,
    pub accounts: AccountTree<AccountTreeNodes, Accounts, TokenTreeNodes, Tokens>,
    pub tsb_infos: TSBInfos,
    pub txs: Txs,
}
impl<
        TsRoot: Value + Default,
        AccountTreeNodes: Array<Option<Fr>> + Default,
        Accounts: Array<Account<TokenTreeNodes, Tokens>> + Default,
        TokenTreeNodes: Array<Option<Fr>> + Default,
        Tokens: Array<Token> + Default,
        TSBInfos: Array<tsb_info::TSBInfo> + Default,
        Txs: Array<Tx> + Default,
    > Default for State<TsRoot, AccountTreeNodes, Accounts, TokenTreeNodes, Tokens, TSBInfos, Txs>
{
    fn default() -> Self {
        Self {
            ts_root: TsRoot::default(),
            accounts: AccountTree::default(),
            tsb_infos: TSBInfos::default(),
            txs: Txs::default(),
        }
    }
}
impl<
        TsRoot: Value,
        AccountTreeNodes: Array<Option<Fr>>,
        Accounts: Array<Account<TokenTreeNodes, Tokens>>,
        TokenTreeNodes: Array<Option<Fr>>,
        Tokens: Array<Token>,
        TSBInfos: Array<tsb_info::TSBInfo>,
        Txs: Array<Tx>,
    > State<TsRoot, AccountTreeNodes, Accounts, TokenTreeNodes, Tokens, TSBInfos, Txs>
{
    pub fn get_root(&self) -> Result<Fr, String> {
        Ok(poseidon::<4>(&[
            Fr::from(2u64),
            self.ts_root.get()?,
            self.accounts.get_root()?,
        ]))
    }
    pub fn set_ts_root(&mut self, ts_root: Fr) -> Result<(), String> {
        self.ts_root.set(&ts_root)
    }
    pub fn push_tx(&mut self, tx_id: usize, raw_tx: RawTx) -> Result<(), String> {
        self.txs.set(tx_id, &Tx::new(raw_tx))
    }
    pub fn update(&mut self, tx_id: usize) -> Result<(), String> {
        let raw_tx = self.txs.get(tx_id)?.raw_tx;
        match raw_tx {
            RawTx::TxNoop(_) => {}
            RawTx::TxRegister(tx) => {
                self.accounts
                    .update(tx.account_id, |acc| Ok(acc.set_l2_addr(tx.hashed_pub_key)?))?;
            }
            RawTx::TxDeposit(tx) => {
                self.accounts.update(tx.account_id, |acc| {
                    Ok(acc.income(tx.deposit_token_id, tx.deposit_amt)?)
                })?;
            }
            RawTx::TxForcedWithdraw(tx) => {
                self.accounts.update(tx.account_id, |acc| {
                    Ok(acc.outgo(tx.withdraw_token_id, tx.withdraw_amt)?)
                })?;
            }
            RawTx::TxTransfer(tx) => {
                self.accounts.update(tx.sender_id, |acc| {
                    acc.outgo(tx.transfer_token_id, tx.transfer_amt)?;
                    acc.increase_nonce()?;
                    Ok(())
                })?;
                self.accounts.update(tx.receiver_id, |acc| {
                    Ok(acc.income(tx.transfer_token_id, tx.transfer_amt)?)
                })?;
            }
            RawTx::TxWithdraw(tx) => {
                self.accounts.update(tx.account_id, |acc| {
                    acc.outgo(tx.withdraw_token_id, tx.withdraw_amt)?;
                    acc.outgo(tx.tx_fee_token_id, tx.tx_fee_amt)?;
                    acc.increase_nonce()?;
                    Ok(())
                })?;
            }
            RawTx::TxCreateTsbBondToken(tx) => {
                self.tsb_infos.set(
                    tx.bond_token_id as usize,
                    &TSBInfo {
                        base_token_id: tx.base_token_id as usize,
                        maturity: tx.maturity,
                    },
                )?;
            }
            RawTx::TxAucLend(tx) => {
                self.accounts.update(tx.lender_id, |acc| {
                    let days_from_matched = calc_days(tx.matched_time, tx.maturity_time);
                    let expected_fee_amt = primary_market::calc_fee(
                        tx.fee_rate,
                        tx.lending_amt,
                        tx.default_matched_interest_rate + mechanism::_1fixed(),
                        days_from_matched,
                    );
                    let amt_to_be_lock = tx.lending_amt
                        + if tx.primary_lend_min_fee_amt > expected_fee_amt {
                            tx.primary_lend_min_fee_amt
                        } else {
                            expected_fee_amt
                        };
                    let mut current_tx = self.txs.get(tx_id)?;
                    current_tx.locked_amt = amt_to_be_lock;
                    self.txs.set(tx_id, &current_tx)?;
                    acc.lock(tx.lending_token_id, amt_to_be_lock)?;
                    Ok(())
                })?;
            }
            RawTx::TxAucBorrow(tx) => {
                self.accounts.update(tx.sender_id, |acc| {
                    let amt_to_be_lock = tx.collateral_amt;
                    acc.lock(tx.collateral_token_id, amt_to_be_lock)?;
                    let mut current_tx = self.txs.get(tx_id)?;
                    current_tx.locked_amt = amt_to_be_lock;
                    self.txs.set(tx_id, &current_tx)?;
                    Ok(())
                })?;
            }
            RawTx::TxAdminCancel(tx) => {
                let mut order = self.txs.get(tx.tx_id as usize)?;
                let (accound_id, token_id) = match order.raw_tx {
                    RawTx::TxAucLend(order) => (order.lender_id, order.lending_token_id),
                    RawTx::TxAucBorrow(order) => (order.sender_id, order.collateral_token_id),
                    RawTx::TxSecLimitOrder(order) => (order.sender_id, order.sell_token_id),
                    RawTx::TxSecMarketOrder(order) => (order.sender_id, order.sell_token_id),
                    _ => {
                        return Err("invalid tx type to cancel".to_string());
                    }
                };
                self.accounts.update(accound_id, |acc| {
                    acc.unlock(token_id, order.locked_amt)?;
                    Ok(())
                })?;
                order.locked_amt = Fr::zero();
                self.txs.set(tx.tx_id as usize, &order)?;
            }
            RawTx::TxUserCancel(tx) => {
                let mut order = self.txs.get(tx.tx_id as usize)?;
                let (accound_id, token_id) = match order.raw_tx {
                    RawTx::TxAucLend(order) => (order.lender_id, order.lending_token_id),
                    RawTx::TxAucBorrow(order) => (order.sender_id, order.collateral_token_id),
                    RawTx::TxSecLimitOrder(order) => (order.sender_id, order.sell_token_id),
                    RawTx::TxSecMarketOrder(order) => (order.sender_id, order.sell_token_id),
                    _ => {
                        return Err(format!("invalid tx type to cancel: {:?}", order.raw_tx));
                    }
                };
                self.accounts.update(accound_id, |acc| {
                    acc.unlock(token_id, order.locked_amt)?;
                    acc.outgo(tx.tx_fee_token_id, tx.tx_fee_amt)?;
                    Ok(())
                })?;
                order.locked_amt = Fr::zero();
                self.txs.set(tx.tx_id as usize, &order)?;
            }
            RawTx::TxAucStart(_) => {}
            RawTx::TxAucMatch(tx) => {
                let (mut borrower, matched_pir, borrower_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxAucStart(tx) => {
                                let borrower_tx_id = tmp - tx.borrower_tx_offset as usize;
                                break (
                                    self.txs.get(borrower_tx_id)?,
                                    tx.ori_matched_interest,
                                    borrower_tx_id,
                                );
                            }
                            RawTx::TxAucMatch(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let (debt_token_id, matched_time, maturity) = {
                    let mut tmp = tx_id + 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxAucEnd(tx) => {
                                break (tx.debt_token_id, tx.matched_time, tx.maturity);
                            }
                            RawTx::TxAucMatch(_) => Ok(tmp += 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let tsb_info = TSBInfo {
                    base_token_id: debt_token_id as usize,
                    maturity,
                };
                let mut bond_token_id: u64 = 0;
                loop {
                    if self.tsb_infos.get(bond_token_id as usize)? == tsb_info {
                        break;
                    }
                    bond_token_id += 1u64;
                }

                let lender_tx_id = tx_id - tx.lender_tx_offset as usize;
                let mut lender = self.txs.get(lender_tx_id)?;
                let (
                    sender_id,
                    lend_token_id,
                    signed_lend_amt,
                    fee_rate,
                    maturity,
                    default_matched_interest_rate,
                    primary_lend_min_fee_amt,
                ) = match lender.raw_tx {
                    RawTx::TxAucLend(tx) => Ok((
                        tx.lender_id,
                        tx.lending_token_id,
                        tx.lending_amt,
                        tx.fee_rate,
                        tx.maturity_time,
                        tx.default_matched_interest_rate,
                        tx.primary_lend_min_fee_amt,
                    )),
                    _ => Err("invalid tx type".to_string()),
                }?;

                let days = calc_days(matched_time, maturity);
                primary_market::mechanism(&mut borrower, &mut lender, days, matched_pir)?;
                let matched_amt = lender.cum_deducted_amt - lender.ori_cum_deducted_amt;
                let matched_tsb_amt = lender.cum_target_amt - lender.ori_cum_target_amt;
                let expected_matched_fee_amt = primary_market::calc_fee(
                    fee_rate,
                    matched_amt,
                    default_matched_interest_rate + mechanism::_1fixed(),
                    days,
                );
                let matched_fee_amt = {
                    let new_credit_amt = if lender.credit_amt > primary_lend_min_fee_amt {
                        lender.credit_amt
                    } else {
                        primary_lend_min_fee_amt
                    };
                    let charged_credit_amt = {
                        let l = new_credit_amt - lender.credit_amt;
                        let r = if new_credit_amt > lender.credit_amt {
                            new_credit_amt - lender.credit_amt
                        } else {
                            Fr::zero()
                        };
                        if l < r {
                            l
                        } else {
                            r
                        }
                    };
                    lender.cum_fee_amt += expected_matched_fee_amt;
                    let charged_fee_amt = {
                        let l = if lender.cum_fee_amt > new_credit_amt {
                            lender.cum_fee_amt - new_credit_amt
                        } else {
                            Fr::zero()
                        };
                        let r = expected_matched_fee_amt;
                        if l < r {
                            l
                        } else {
                            r
                        }
                    };
                    lender.credit_amt = new_credit_amt;
                    charged_fee_amt + charged_credit_amt
                };

                lender.locked_amt -= matched_amt + matched_fee_amt;
                self.accounts.update(sender_id, |acc| {
                    acc.deduct(lend_token_id, matched_amt + matched_fee_amt)?;
                    acc.income(bond_token_id, matched_tsb_amt)?;
                    Ok(())
                })?;
                lender.ori_cum_deducted_amt = lender.cum_deducted_amt;
                lender.ori_cum_target_amt = lender.cum_target_amt;

                if lender.cum_deducted_amt == signed_lend_amt {
                    self.accounts.update(sender_id, |acc| {
                        acc.unlock(lend_token_id, lender.locked_amt)?;
                        Ok(())
                    })?;
                    lender.locked_amt = Fr::zero();
                }

                self.txs.set(lender_tx_id, &lender)?;
                self.txs.set(borrower_tx_id, &borrower)?;
            }
            RawTx::TxAucEnd(tx) => {
                let (mut borrower, matched_pir, borrower_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxAucStart(tx) => {
                                let borrower_tx_id = tmp - tx.borrower_tx_offset as usize;
                                break (
                                    self.txs.get(borrower_tx_id)?,
                                    tx.ori_matched_interest,
                                    borrower_tx_id,
                                );
                            }
                            RawTx::TxAucMatch(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let matched_amt = borrower.cum_target_amt - borrower.ori_cum_target_amt;
                let matched_collateral_amt =
                    borrower.cum_deducted_amt - borrower.ori_cum_deducted_amt;
                let (borrower_id, signed_collateral_amt, fee_rate, primary_borrow_min_fee_amt) =
                    match borrower.raw_tx {
                        RawTx::TxAucBorrow(tx) => Ok((
                            tx.sender_id,
                            tx.collateral_amt,
                            tx.fee_rate,
                            tx.primary_borrow_min_fee_amt,
                        )),
                        _ => Err("invalid tx type".to_string()),
                    }?;
                let (days, debt_token_id) = match self.txs.get(tx_id - 1)?.raw_tx {
                    RawTx::TxAucMatch(order) => {
                        let order = self.txs.get(tx_id - 1 - order.lender_tx_offset as usize)?;
                        let (maturity_time, lending_token_id) = match order.raw_tx {
                            RawTx::TxAucLend(tx) => Ok((tx.maturity_time, tx.lending_token_id)),
                            _ => Err("invalid tx type".to_string()),
                        }?;
                        Ok((calc_days(tx.matched_time, maturity_time), lending_token_id))
                    }
                    RawTx::TxAucStart(_) => Ok((Fr::zero(), 0)),
                    _ => Err("invalid tx type".to_string()),
                }?;
                let expected_matched_fee_amt =
                    primary_market::calc_fee(fee_rate, matched_amt, matched_pir, days);
                let matched_fee_amt = {
                    let min_fee = if borrower.credit_amt > primary_borrow_min_fee_amt {
                        borrower.credit_amt
                    } else {
                        primary_borrow_min_fee_amt
                    };
                    let new_credit_amt = {
                        let l = min_fee;
                        let tmp = if borrower.credit_amt > borrower.cum_fee_amt {
                            borrower.credit_amt
                        } else {
                            borrower.cum_fee_amt
                        };
                        let r = tmp + matched_amt;
                        if l < r {
                            l
                        } else {
                            r
                        }
                    };
                    let charged_credit_amt = {
                        let l = new_credit_amt - borrower.credit_amt;
                        let r = if new_credit_amt > borrower.credit_amt {
                            new_credit_amt - borrower.credit_amt
                        } else {
                            Fr::zero()
                        };
                        if l < r {
                            l
                        } else {
                            r
                        }
                    };
                    borrower.cum_fee_amt += expected_matched_fee_amt;
                    let charged_fee_amt = {
                        let l = if borrower.cum_fee_amt > new_credit_amt {
                            borrower.cum_fee_amt - new_credit_amt
                        } else {
                            Fr::zero()
                        };
                        let r = expected_matched_fee_amt;
                        if l < r {
                            l
                        } else {
                            r
                        }
                    };
                    borrower.credit_amt = new_credit_amt;
                    charged_fee_amt + charged_credit_amt
                };
                borrower.locked_amt -= matched_collateral_amt;

                self.accounts.update(borrower_id, |acc| {
                    acc.deduct(tx.collateral_token_id, matched_collateral_amt)?;
                    acc.income(debt_token_id, matched_amt - matched_fee_amt)?;
                    Ok(())
                })?;
                borrower.ori_cum_deducted_amt = borrower.cum_deducted_amt;
                borrower.ori_cum_target_amt = borrower.cum_target_amt;

                if borrower.cum_deducted_amt == signed_collateral_amt {
                    self.accounts.update(borrower_id, |acc| {
                        acc.unlock(tx.collateral_token_id, borrower.locked_amt)?;
                        Ok(())
                    })?;
                    borrower.locked_amt = Fr::zero();
                }

                self.txs.set(borrower_tx_id, &borrower)?;
            }
            RawTx::TxSecLimitOrder(tx) => {
                self.accounts.update(tx.sender_id, |acc| {
                    let mut tsb_info_leaf = self.tsb_infos.get(tx.buy_token_id as usize)?;
                    let side = tsb_info_leaf.base_token_id == TSBInfo::default().base_token_id;
                    if side {
                        tsb_info_leaf = self.tsb_infos.get(tx.sell_token_id as usize)?;
                    }
                    let maturity = tsb_info_leaf.maturity;
                    let days_from_matched = calc_days(tx.matched_time, maturity);
                    let days_from_expired = calc_days(tx.expired_time, maturity);
                    let amt_to_be_lock = match side {
                        true => tx.sell_amt,
                        false => {
                            secondary_market::calc_new_bq(
                                tx.buy_amt,
                                tx.buy_amt,
                                tx.sell_amt,
                                if tx.buy_amt < tx.sell_amt {
                                    days_from_matched
                                } else {
                                    days_from_expired
                                },
                            ) + {
                                let expected_fee_amt = secondary_market::calc_fee(
                                    tx.buy_amt,
                                    if tx.fee0 < tx.fee1 { tx.fee1 } else { tx.fee0 },
                                    days_from_matched,
                                );
                                let max_min_fee = if tx.secondary_taker_min_fee_amt
                                    > tx.secondary_maker_min_fee_amt
                                {
                                    tx.secondary_taker_min_fee_amt
                                } else {
                                    tx.secondary_maker_min_fee_amt
                                };
                                if expected_fee_amt > max_min_fee {
                                    expected_fee_amt
                                } else {
                                    max_min_fee
                                }
                            }
                        }
                    };
                    let mut current_tx = self.txs.get(tx_id)?;
                    current_tx.locked_amt = amt_to_be_lock;
                    self.txs.set(tx_id, &current_tx)?;
                    acc.lock(tx.sell_token_id, amt_to_be_lock)?;
                    Ok(())
                })?;
            }
            RawTx::TxSecLimitStart(_) => {}
            RawTx::TxSecLimitExchange(tx) => {
                let (mut taker, taker_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecLimitStart(tx) => {
                                let taker_tx_id = tmp - tx.taker_tx_offset as usize;
                                break (self.txs.get(taker_tx_id)?, taker_tx_id);
                            }
                            RawTx::TxSecLimitExchange(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let matched_time = {
                    let mut tmp = tx_id + 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecLimitEnd(tx) => {
                                break tx.matched_time;
                            }
                            RawTx::TxSecLimitExchange(_) => Ok(tmp += 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let maker_tx_id = tx_id - tx.maker_tx_offset as usize;
                let mut maker = self.txs.get(maker_tx_id)?;
                let (
                    maker_acc_id,
                    sell_token_id,
                    signed_sell_amt,
                    buy_token_id,
                    signed_buy_amt,
                    maker_side,
                    days,
                    fee_rate,
                    secondary_maker_min_fee_amt,
                ) = match maker.raw_tx {
                    RawTx::TxSecLimitOrder(tx) => {
                        let mut tsb_info_leaf = self.tsb_infos.get(tx.buy_token_id as usize)?;
                        let side = tsb_info_leaf.base_token_id == TSBInfo::default().base_token_id;
                        if side {
                            tsb_info_leaf = self.tsb_infos.get(tx.sell_token_id as usize)?;
                        }
                        let maturity = tsb_info_leaf.maturity;
                        let days = calc_days(matched_time, maturity);
                        Ok((
                            tx.sender_id,
                            tx.sell_token_id,
                            tx.sell_amt,
                            tx.buy_token_id,
                            tx.buy_amt,
                            side,
                            days,
                            tx.fee1,
                            tx.secondary_maker_min_fee_amt,
                        ))
                    }
                    _ => Err("invalid tx type".to_string()),
                }?;
                secondary_market::mechanism(&mut taker, &mut maker, days, maker_side)?;
                let matched_sell_amt = maker.cum_deducted_amt - maker.ori_cum_deducted_amt;
                let matched_buy_amt = maker.cum_target_amt - maker.ori_cum_target_amt;
                let (fee_from_sell_amt, fee_from_buy_amt) = if maker_side {
                    (Fr::zero(), {
                        let expected_matched_fee_amt =
                            secondary_market::calc_fee(fee_rate, matched_sell_amt, days);
                        let min_fee = if maker.credit_amt > secondary_maker_min_fee_amt {
                            maker.credit_amt
                        } else {
                            secondary_maker_min_fee_amt
                        };
                        let new_credit_amt = {
                            let l = min_fee;
                            let tmp = if maker.credit_amt > maker.cum_fee_amt {
                                maker.credit_amt
                            } else {
                                maker.cum_fee_amt
                            };
                            let r = tmp + matched_buy_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        let charged_credit_amt = {
                            let l = new_credit_amt - maker.credit_amt;
                            let r = if new_credit_amt > maker.credit_amt {
                                new_credit_amt - maker.credit_amt
                            } else {
                                Fr::zero()
                            };
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        maker.cum_fee_amt += expected_matched_fee_amt;
                        let charged_fee_amt = {
                            let l = if maker.cum_fee_amt > new_credit_amt {
                                maker.cum_fee_amt - new_credit_amt
                            } else {
                                Fr::zero()
                            };
                            let r = expected_matched_fee_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        maker.credit_amt = new_credit_amt;
                        charged_fee_amt + charged_credit_amt
                    })
                } else {
                    (
                        {
                            let expected_matched_fee_amt =
                                secondary_market::calc_fee(fee_rate, matched_buy_amt, days);
                            let new_credit_amt = if maker.credit_amt > secondary_maker_min_fee_amt {
                                maker.credit_amt
                            } else {
                                secondary_maker_min_fee_amt
                            };
                            let charged_credit_amt = {
                                let l = new_credit_amt - maker.credit_amt;
                                let r = if new_credit_amt > maker.credit_amt {
                                    new_credit_amt - maker.credit_amt
                                } else {
                                    Fr::zero()
                                };
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            maker.cum_fee_amt += expected_matched_fee_amt;
                            let charged_fee_amt = {
                                let l = if maker.cum_fee_amt > new_credit_amt {
                                    maker.cum_fee_amt - new_credit_amt
                                } else {
                                    Fr::zero()
                                };
                                let r = expected_matched_fee_amt;
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            maker.credit_amt = new_credit_amt;
                            charged_fee_amt + charged_credit_amt
                        },
                        Fr::zero(),
                    )
                };
                maker.locked_amt -= matched_sell_amt + fee_from_sell_amt;

                self.accounts.update(maker_acc_id, |acc| {
                    acc.deduct(sell_token_id, matched_sell_amt + fee_from_sell_amt)?;
                    acc.income(buy_token_id, matched_buy_amt - fee_from_buy_amt)?;
                    Ok(())
                })?;
                maker.ori_cum_deducted_amt = maker.cum_deducted_amt;
                maker.ori_cum_target_amt = maker.cum_target_amt;

                if {
                    if maker_side {
                        maker.cum_deducted_amt == signed_sell_amt
                    } else {
                        maker.cum_target_amt == signed_buy_amt
                    }
                } {
                    self.accounts.update(maker_acc_id, |acc| {
                        acc.unlock(sell_token_id, maker.locked_amt)?;
                        Ok(())
                    })?;
                    maker.locked_amt = Fr::zero();
                }

                self.txs.set(maker_tx_id, &maker)?;
                self.txs.set(taker_tx_id, &taker)?;
            }
            RawTx::TxSecLimitEnd(tx) => {
                let (mut taker, taker_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecLimitStart(tx) => {
                                let taker_tx_id = tmp - tx.taker_tx_offset as usize;
                                break (self.txs.get(taker_tx_id)?, taker_tx_id);
                            }
                            RawTx::TxSecLimitExchange(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let matched_sell_amt = taker.cum_deducted_amt - taker.ori_cum_deducted_amt;
                let matched_buy_amt = taker.cum_target_amt - taker.ori_cum_target_amt;
                let matched_time = tx.matched_time;
                let (
                    taker_acc_id,
                    sell_token_id,
                    signed_sell_amt,
                    buy_token_id,
                    signed_buy_amt,
                    side,
                    days,
                    fee_rate,
                    secondary_taker_min_fee_amt,
                ) = match taker.raw_tx {
                    RawTx::TxSecLimitOrder(tx) => {
                        let mut tsb_info_leaf = self.tsb_infos.get(tx.buy_token_id as usize)?;
                        let side = tsb_info_leaf.base_token_id == TSBInfo::default().base_token_id;
                        if side {
                            tsb_info_leaf = self.tsb_infos.get(tx.sell_token_id as usize)?;
                        }
                        let maturity = tsb_info_leaf.maturity;
                        let days = calc_days(matched_time, maturity);
                        Ok((
                            tx.sender_id,
                            tx.sell_token_id,
                            tx.sell_amt,
                            tx.buy_token_id,
                            tx.buy_amt,
                            side,
                            days,
                            tx.fee0,
                            tx.secondary_taker_min_fee_amt,
                        ))
                    }
                    _ => Err("invalid tx type".to_string()),
                }?;

                let (fee_from_sell_amt, fee_from_buy_amt) = if side {
                    (Fr::zero(), {
                        let expected_matched_fee_amt =
                            secondary_market::calc_fee(fee_rate, matched_sell_amt, days);
                        let min_fee = if taker.credit_amt > secondary_taker_min_fee_amt {
                            taker.credit_amt
                        } else {
                            secondary_taker_min_fee_amt
                        };
                        let new_credit_amt = {
                            let l = min_fee;
                            let tmp = if taker.credit_amt > taker.cum_fee_amt {
                                taker.credit_amt
                            } else {
                                taker.cum_fee_amt
                            };
                            let r = tmp + matched_buy_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        let charged_credit_amt = {
                            let l = new_credit_amt - taker.credit_amt;
                            let r = if new_credit_amt > taker.credit_amt {
                                new_credit_amt - taker.credit_amt
                            } else {
                                Fr::zero()
                            };
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        taker.cum_fee_amt += expected_matched_fee_amt;
                        let charged_fee_amt = {
                            let l = if taker.cum_fee_amt > new_credit_amt {
                                taker.cum_fee_amt - new_credit_amt
                            } else {
                                Fr::zero()
                            };
                            let r = expected_matched_fee_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        taker.credit_amt = new_credit_amt;
                        charged_fee_amt + charged_credit_amt
                    })
                } else {
                    (
                        {
                            let expected_matched_fee_amt =
                                secondary_market::calc_fee(fee_rate, matched_buy_amt, days);
                            let new_credit_amt = if taker.credit_amt > secondary_taker_min_fee_amt {
                                taker.credit_amt
                            } else {
                                secondary_taker_min_fee_amt
                            };
                            let charged_credit_amt = {
                                let l = new_credit_amt - taker.credit_amt;
                                let r = if new_credit_amt > taker.credit_amt {
                                    new_credit_amt - taker.credit_amt
                                } else {
                                    Fr::zero()
                                };
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            taker.cum_fee_amt += expected_matched_fee_amt;
                            let charged_fee_amt = {
                                let l = if taker.cum_fee_amt > new_credit_amt {
                                    taker.cum_fee_amt - new_credit_amt
                                } else {
                                    Fr::zero()
                                };
                                let r = expected_matched_fee_amt;
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            taker.credit_amt = new_credit_amt;
                            charged_fee_amt + charged_credit_amt
                        },
                        Fr::zero(),
                    )
                };

                taker.locked_amt -= matched_sell_amt + fee_from_sell_amt;
                self.accounts.update(taker_acc_id, |acc| {
                    acc.deduct(sell_token_id, matched_sell_amt + fee_from_sell_amt)?;
                    acc.income(buy_token_id, matched_buy_amt - fee_from_buy_amt)?;
                    Ok(())
                })?;
                taker.ori_cum_deducted_amt = taker.cum_deducted_amt;
                taker.ori_cum_target_amt = taker.cum_target_amt;

                if {
                    if side {
                        taker.cum_deducted_amt == signed_sell_amt
                    } else {
                        taker.cum_target_amt == signed_buy_amt
                    }
                } {
                    self.accounts.update(taker_acc_id, |acc| {
                        acc.unlock(sell_token_id, taker.locked_amt)?;
                        Ok(())
                    })?;
                    taker.locked_amt = Fr::zero();
                }

                self.txs.set(taker_tx_id, &taker)?;
            }
            RawTx::TxIncreaseEpoch(_) => {}
            RawTx::TxRedeem(tx) => {
                let base_token_id = self.tsb_infos.get(tx.token_id as usize)?.base_token_id as u64;
                self.accounts.update(tx.sender_id, |acc| {
                    acc.outgo(tx.token_id, tx.amount)?;
                    acc.income(base_token_id, tx.amount)?;
                    acc.increase_nonce()?;
                    Ok(())
                })?;
            }
            RawTx::TxWithdrawFee(_) => {}
            RawTx::TxSecMarketOrder(_) => {}
            RawTx::TxSecMarketExchange(tx) => {
                let (mut taker, taker_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecMarketOrder(_) => {
                                break (order, tmp);
                            }
                            RawTx::TxSecMarketExchange(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let matched_time = {
                    let mut tmp = tx_id + 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecMarketEnd(tx) => {
                                break tx.matched_time;
                            }
                            RawTx::TxSecMarketExchange(_) => Ok(tmp += 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let maker_tx_id = tx_id - tx.maker_tx_offset as usize;
                let mut maker = self.txs.get(maker_tx_id)?;
                let (
                    maker_acc_id,
                    sell_token_id,
                    signed_sell_amt,
                    buy_token_id,
                    signed_buy_amt,
                    maker_side,
                    days,
                    fee_rate,
                    secondary_maker_min_fee_amt,
                ) = match maker.raw_tx {
                    RawTx::TxSecLimitOrder(tx) => {
                        let mut tsb_info_leaf = self.tsb_infos.get(tx.buy_token_id as usize)?;
                        let side = tsb_info_leaf.base_token_id == TSBInfo::default().base_token_id;
                        if side {
                            tsb_info_leaf = self.tsb_infos.get(tx.sell_token_id as usize)?;
                        }
                        let maturity = tsb_info_leaf.maturity;
                        let days = calc_days(matched_time, maturity);
                        Ok((
                            tx.sender_id,
                            tx.sell_token_id,
                            tx.sell_amt,
                            tx.buy_token_id,
                            tx.buy_amt,
                            side,
                            days,
                            tx.fee1,
                            tx.secondary_maker_min_fee_amt,
                        ))
                    }
                    _ => Err("invalid tx type".to_string()),
                }?;
                secondary_market::mechanism(&mut taker, &mut maker, days, maker_side)?;
                let matched_sell_amt = maker.cum_deducted_amt - maker.ori_cum_deducted_amt;
                let matched_buy_amt = maker.cum_target_amt - maker.ori_cum_target_amt;
                let (fee_from_sell_amt, fee_from_buy_amt) = if maker_side {
                    (Fr::zero(), {
                        let expected_matched_fee_amt =
                            secondary_market::calc_fee(fee_rate, matched_sell_amt, days);
                        let min_fee = if maker.credit_amt > secondary_maker_min_fee_amt {
                            maker.credit_amt
                        } else {
                            secondary_maker_min_fee_amt
                        };
                        let new_credit_amt = {
                            let l = min_fee;
                            let tmp = if maker.credit_amt > maker.cum_fee_amt {
                                maker.credit_amt
                            } else {
                                maker.cum_fee_amt
                            };
                            let r = tmp + matched_buy_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        let charged_credit_amt = {
                            let l = new_credit_amt - maker.credit_amt;
                            let r = if new_credit_amt > maker.credit_amt {
                                new_credit_amt - maker.credit_amt
                            } else {
                                Fr::zero()
                            };
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        maker.cum_fee_amt += expected_matched_fee_amt;
                        let charged_fee_amt = {
                            let l = if maker.cum_fee_amt > new_credit_amt {
                                maker.cum_fee_amt - new_credit_amt
                            } else {
                                Fr::zero()
                            };
                            let r = expected_matched_fee_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        maker.credit_amt = new_credit_amt;
                        charged_fee_amt + charged_credit_amt
                    })
                } else {
                    (
                        {
                            let expected_matched_fee_amt =
                                secondary_market::calc_fee(fee_rate, matched_buy_amt, days);
                            let new_credit_amt = if maker.credit_amt > secondary_maker_min_fee_amt {
                                maker.credit_amt
                            } else {
                                secondary_maker_min_fee_amt
                            };
                            let charged_credit_amt = {
                                let l = new_credit_amt - maker.credit_amt;
                                let r = if new_credit_amt > maker.credit_amt {
                                    new_credit_amt - maker.credit_amt
                                } else {
                                    Fr::zero()
                                };
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            maker.cum_fee_amt += expected_matched_fee_amt;
                            let charged_fee_amt = {
                                let l = if maker.cum_fee_amt > new_credit_amt {
                                    maker.cum_fee_amt - new_credit_amt
                                } else {
                                    Fr::zero()
                                };
                                let r = expected_matched_fee_amt;
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            maker.credit_amt = new_credit_amt;
                            charged_fee_amt + charged_credit_amt
                        },
                        Fr::zero(),
                    )
                };
                maker.locked_amt -= matched_sell_amt + fee_from_sell_amt;

                self.accounts.update(maker_acc_id, |acc| {
                    acc.deduct(sell_token_id, matched_sell_amt + fee_from_sell_amt)?;
                    acc.income(buy_token_id, matched_buy_amt - fee_from_buy_amt)?;
                    Ok(())
                })?;
                maker.ori_cum_deducted_amt = maker.cum_deducted_amt;
                maker.ori_cum_target_amt = maker.cum_target_amt;

                if {
                    if maker_side {
                        maker.cum_deducted_amt == signed_sell_amt
                    } else {
                        maker.cum_target_amt == signed_buy_amt
                    }
                } {
                    self.accounts.update(maker_acc_id, |acc| {
                        acc.unlock(sell_token_id, maker.locked_amt)?;
                        Ok(())
                    })?;
                    maker.locked_amt = Fr::zero();
                }

                self.txs.set(maker_tx_id, &maker)?;
                self.txs.set(taker_tx_id, &taker)?;
            }
            RawTx::TxSecMarketEnd(tx) => {
                let (mut taker, taker_tx_id) = {
                    let mut tmp = tx_id - 1;
                    loop {
                        let order = self.txs.get(tmp)?;
                        match order.raw_tx {
                            RawTx::TxSecMarketOrder(_) => {
                                break (order, tmp);
                            }
                            RawTx::TxSecMarketExchange(_) => Ok(tmp -= 1),
                            _ => Err("invalid tx type".to_string()),
                        }?
                    }
                };
                let matched_sell_amt = taker.cum_deducted_amt - taker.ori_cum_deducted_amt;
                let matched_buy_amt = taker.cum_target_amt - taker.ori_cum_target_amt;
                let matched_time = tx.matched_time;
                let (
                    taker_acc_id,
                    sell_token_id,
                    buy_token_id,
                    side,
                    days,
                    fee_rate,
                    secondary_taker_min_fee_amt,
                ) = match taker.raw_tx {
                    RawTx::TxSecMarketOrder(tx) => {
                        let mut tsb_info_leaf = self.tsb_infos.get(tx.buy_token_id as usize)?;
                        let side = tsb_info_leaf.base_token_id == TSBInfo::default().base_token_id;
                        if side {
                            tsb_info_leaf = self.tsb_infos.get(tx.sell_token_id as usize)?;
                        }
                        let maturity = tsb_info_leaf.maturity;
                        let days = calc_days(matched_time, maturity);
                        Ok((
                            tx.sender_id,
                            tx.sell_token_id,
                            tx.buy_token_id,
                            side,
                            days,
                            tx.fee0,
                            tx.secondary_taker_min_fee_amt,
                        ))
                    }
                    _ => Err("invalid tx type".to_string()),
                }?;

                let (fee_from_sell_amt, fee_from_buy_amt) = if side {
                    (Fr::zero(), {
                        let expected_matched_fee_amt =
                            secondary_market::calc_fee(fee_rate, matched_sell_amt, days);
                        let min_fee = if taker.credit_amt > secondary_taker_min_fee_amt {
                            taker.credit_amt
                        } else {
                            secondary_taker_min_fee_amt
                        };
                        let new_credit_amt = {
                            let l = min_fee;
                            let tmp = if taker.credit_amt > taker.cum_fee_amt {
                                taker.credit_amt
                            } else {
                                taker.cum_fee_amt
                            };
                            let r = tmp + matched_buy_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        let charged_credit_amt = {
                            let l = new_credit_amt - taker.credit_amt;
                            let r = if new_credit_amt > taker.credit_amt {
                                new_credit_amt - taker.credit_amt
                            } else {
                                Fr::zero()
                            };
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        taker.cum_fee_amt += expected_matched_fee_amt;
                        let charged_fee_amt = {
                            let l = if taker.cum_fee_amt > new_credit_amt {
                                taker.cum_fee_amt - new_credit_amt
                            } else {
                                Fr::zero()
                            };
                            let r = expected_matched_fee_amt;
                            if l < r {
                                l
                            } else {
                                r
                            }
                        };
                        taker.credit_amt = new_credit_amt;
                        charged_fee_amt + charged_credit_amt
                    })
                } else {
                    (
                        {
                            let expected_matched_fee_amt =
                                secondary_market::calc_fee(fee_rate, matched_buy_amt, days);
                            let new_credit_amt = if taker.credit_amt > secondary_taker_min_fee_amt {
                                taker.credit_amt
                            } else {
                                secondary_taker_min_fee_amt
                            };
                            let charged_credit_amt = {
                                let l = new_credit_amt - taker.credit_amt;
                                let r = if new_credit_amt > taker.credit_amt {
                                    new_credit_amt - taker.credit_amt
                                } else {
                                    Fr::zero()
                                };
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            taker.cum_fee_amt += expected_matched_fee_amt;
                            let charged_fee_amt = {
                                let l = if taker.cum_fee_amt > new_credit_amt {
                                    taker.cum_fee_amt - new_credit_amt
                                } else {
                                    Fr::zero()
                                };
                                let r = expected_matched_fee_amt;
                                if l < r {
                                    l
                                } else {
                                    r
                                }
                            };
                            taker.credit_amt = new_credit_amt;
                            charged_fee_amt + charged_credit_amt
                        },
                        Fr::zero(),
                    )
                };

                self.accounts.update(taker_acc_id, |acc| {
                    acc.outgo(sell_token_id, matched_sell_amt + fee_from_sell_amt)?;
                    acc.income(buy_token_id, matched_buy_amt - fee_from_buy_amt)?;
                    Ok(())
                })?;
                taker.ori_cum_deducted_amt = taker.cum_deducted_amt;
                taker.ori_cum_target_amt = taker.cum_target_amt;

                self.txs.set(taker_tx_id, &taker)?;
            }
            RawTx::TxEvacuation(tx) => {
                self.accounts.update(tx.sender_id, |acc| {
                    acc.outgo(tx.token_id, tx.amount)?;
                    Ok(())
                })?;
            }
            RawTx::TxSetAdminTsAddr(_) => {}
        }
        Ok(())
    }
}
