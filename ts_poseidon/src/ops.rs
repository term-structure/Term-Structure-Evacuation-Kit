use super::constant::{c::C, s::S};
use ark_bn254::Fr;
use ark_ff::Field;
use num_traits::identities::Zero;

pub fn sigma(x: Fr) -> Fr {
    x.pow([5u64])
}
pub fn ark<const INPUT_LEN_PLUS_ONE: usize>(
    mut x: [Fr; INPUT_LEN_PLUS_ONE],
    r: usize,
) -> [Fr; INPUT_LEN_PLUS_ONE] {
    for i in 0..INPUT_LEN_PLUS_ONE {
        x[i] += C[INPUT_LEN_PLUS_ONE - 2][i + r];
    }
    x
}
pub fn mix<const INPUT_LEN_PLUS_ONE: usize>(
    x: &[Fr; INPUT_LEN_PLUS_ONE],
    m: &[&[&[Fr]]; 16],
) -> [Fr; INPUT_LEN_PLUS_ONE] {
    let mut y = [Fr::zero(); INPUT_LEN_PLUS_ONE];
    for i in 0..INPUT_LEN_PLUS_ONE {
        y[i] = mix_last(x, i, m);
    }
    y
}
pub fn mix_last<const INPUT_LEN_PLUS_ONE: usize>(
    x: &[Fr; INPUT_LEN_PLUS_ONE],
    s: usize,
    m: &[&[&[Fr]]; 16],
) -> Fr {
    x.iter()
        .enumerate()
        .map(|(i, xi)| m[INPUT_LEN_PLUS_ONE - 2][i][s] * xi)
        .reduce(|a, b| a + b)
        .unwrap()
}
pub fn mix_s<const INPUT_LEN_PLUS_ONE: usize>(
    x: &[Fr; INPUT_LEN_PLUS_ONE],
    r: usize,
) -> [Fr; INPUT_LEN_PLUS_ONE] {
    let mut y = [Fr::zero(); INPUT_LEN_PLUS_ONE];
    y[0] = x
        .iter()
        .enumerate()
        .map(|(i, xi)| S[INPUT_LEN_PLUS_ONE - 2][(INPUT_LEN_PLUS_ONE * 2 - 1) * r + i] * xi)
        .reduce(|a, b| a + b)
        .unwrap();
    for i in 1..INPUT_LEN_PLUS_ONE {
        y[i] = x[i]
            + x[0]
                * S[INPUT_LEN_PLUS_ONE - 2]
                    [(INPUT_LEN_PLUS_ONE * 2 - 1) * r + INPUT_LEN_PLUS_ONE + i - 1];
    }
    y
}
