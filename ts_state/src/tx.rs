use ark_bn254::Fr;
use num_traits::Zero;
use ts_tx::Tx as RawTx;

#[derive(Clone, Copy, Debug)]
pub struct Tx {
    pub raw_tx: RawTx,
    pub cum_deducted_amt: Fr,
    pub cum_target_amt: Fr,
    pub ori_cum_deducted_amt: Fr,
    pub ori_cum_target_amt: Fr,
    pub locked_amt: Fr,
    pub cum_fee_amt: Fr,
    pub credit_amt: Fr,
}
impl Default for Tx {
    fn default() -> Self {
        Tx {
            raw_tx: RawTx::default(),
            cum_deducted_amt: Fr::zero(),
            cum_target_amt: Fr::zero(),
            ori_cum_deducted_amt: Fr::zero(),
            ori_cum_target_amt: Fr::zero(),
            locked_amt: Fr::zero(),
            cum_fee_amt: Fr::zero(),
            credit_amt: Fr::zero(),
        }
    }
}
impl Tx {
    pub fn new(raw_tx: RawTx) -> Self {
        Tx {
            raw_tx,
            cum_deducted_amt: Fr::zero(),
            cum_target_amt: Fr::zero(),
            ori_cum_deducted_amt: Fr::zero(),
            ori_cum_target_amt: Fr::zero(),
            locked_amt: Fr::zero(),
            cum_fee_amt: Fr::zero(),
            credit_amt: Fr::zero(),
        }
    }
}
