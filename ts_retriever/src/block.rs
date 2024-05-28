use serde::Serialize;
use web3::ethabi::{Bytes, FixedBytes, Token, Uint};

#[derive(Serialize)]
pub struct Block {
    pub block_number: Uint,
    pub new_state_root: FixedBytes,
    pub new_ts_root: FixedBytes,
    pub timestamp: Uint,
    pub chunk_id_deltas: Vec<Uint>,
    pub public_data: Bytes,
}
impl Block {
    pub fn from_token(token: &Token) -> Option<Self> {
        if let Token::Tuple(tuple) = token {
            let block_number = if let Token::Uint(block_number) = tuple[0].clone() {
                block_number
            } else {
                return None;
            };
            let new_state_root = if let Token::FixedBytes(new_state_root) = tuple[1].clone() {
                new_state_root
            } else {
                return None;
            };
            let new_ts_root = if let Token::FixedBytes(new_ts_root) = tuple[2].clone() {
                new_ts_root
            } else {
                return None;
            };
            let timestamp = if let Token::Uint(timestamp) = tuple[3].clone() {
                timestamp
            } else {
                return None;
            };
            let chunk_id_deltas = if let Token::Array(chunk_id_deltas) = tuple[4].clone() {
                chunk_id_deltas
                    .into_iter()
                    .map(|token| {
                        if let Token::Uint(chunk_id_delta) = token {
                            chunk_id_delta
                        } else {
                            unreachable!()
                        }
                    })
                    .collect()
            } else {
                return None;
            };
            let public_data = if let Token::Bytes(public_data) = tuple[5].clone() {
                public_data
            } else {
                return None;
            };
            Some(Block {
                block_number,
                new_state_root,
                new_ts_root,
                timestamp,
                chunk_id_deltas,
                public_data,
            })
        } else {
            None
        }
    }
}
