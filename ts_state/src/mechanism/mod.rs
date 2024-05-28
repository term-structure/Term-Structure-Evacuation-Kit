use num_bigint::BigUint;

pub mod primary_market;
pub mod secondary_market;

#[inline]
pub fn _1fixed() -> ark_bn254::Fr {
    ark_bn254::Fr::from(100000000u64)
}

#[inline]
pub fn _365f() -> ark_bn254::Fr {
    ark_bn254::Fr::from(365u64)
}

#[inline]
pub fn _1f() -> ark_bn254::Fr {
    <ark_bn254::Fr as num_traits::One>::one()
}

#[inline]
pub fn calc_days(start_time: ark_bn254::Fr, end_time: ark_bn254::Fr) -> ark_bn254::Fr {
    let start_time_biguint: BigUint = start_time.into();
    let end_time_biguint: BigUint = end_time.into();
    let days = ((end_time_biguint + 86400u64 - 1u64) - start_time_biguint) / 86400u64;
    days.into()
}
