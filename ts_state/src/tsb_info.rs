use ark_bn254::Fr;
use num_traits::Zero;

#[derive(Clone, Copy, Debug)]
pub struct TSBInfo {
    pub base_token_id: usize,
    pub maturity: Fr,
}
impl Default for TSBInfo {
    fn default() -> Self {
        Self {
            base_token_id: 0,
            maturity: Fr::zero(),
        }
    }
}
impl std::cmp::PartialEq for TSBInfo {
    fn eq(&self, other: &Self) -> bool {
        self.base_token_id == other.base_token_id && self.maturity == other.maturity
    }
}
impl std::cmp::Eq for TSBInfo {}