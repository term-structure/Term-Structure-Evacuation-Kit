use self::constant::{c::C, m::M, p::P, N_ROUNDS_F, N_ROUNDS_PS};
use self::ops::{ark, mix, mix_last, mix_s, sigma};
use ark_bn254::Fr;
use num_traits::identities::Zero;

pub mod constant;
pub mod ops;

pub fn poseidon<const INPUT_LEN_PLUS_ONE: usize>(input: &[Fr]) -> Fr {
    let mut state = [Fr::zero(); INPUT_LEN_PLUS_ONE];
    state.iter_mut().enumerate().for_each(|(i, s)| {
        *s = match i {
            0 => Fr::zero(),
            _ => input[i - 1],
        }
    });
    state = ops::ark(state, 0);

    for i in 0..(N_ROUNDS_F / 2 - 1) {
        for j in 0..INPUT_LEN_PLUS_ONE {
            state[j] = sigma(state[j]);
        }
        state = ark(state, (i + 1) * INPUT_LEN_PLUS_ONE);
        state = mix(&state, &M);
    }

    for j in 0..INPUT_LEN_PLUS_ONE {
        state[j] = sigma(state[j]);
    }
    state = ark(state, 4 * INPUT_LEN_PLUS_ONE);
    state = mix(&state, &P);
    for i in 0..N_ROUNDS_PS[INPUT_LEN_PLUS_ONE - 2] {
        state[0] = sigma(state[0]);
        state[0] += C[INPUT_LEN_PLUS_ONE - 2][(N_ROUNDS_F / 2 + 1) * INPUT_LEN_PLUS_ONE + i];
        state = mix_s(&state, i);
    }
    for i in 0..(N_ROUNDS_F / 2 - 1) {
        for j in 0..INPUT_LEN_PLUS_ONE {
            state[j] = sigma(state[j]);
        }
        state = ark(
            state,
            (N_ROUNDS_F / 2 + 1) * INPUT_LEN_PLUS_ONE
                + i * INPUT_LEN_PLUS_ONE
                + N_ROUNDS_PS[INPUT_LEN_PLUS_ONE - 2],
        );
        state = mix(&state, &M);
    }
    for j in 0..INPUT_LEN_PLUS_ONE {
        state[j] = sigma(state[j]);
    }
    mix_last(&state, 0, &M)
}
