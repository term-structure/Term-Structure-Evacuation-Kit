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

#[derive(Serialize, Debug)]
pub struct ExecutedBlock {
    pub block_number: ethabi::Uint,
    pub l1_request_count: ethabi::Uint,
    pub pending_rollup_tx_hash: ethabi::FixedBytes,
    pub commitment: ethabi::FixedBytes,
    pub state_root: ethabi::FixedBytes,
    pub timestamp: ethabi::Uint,
}
impl ExecutedBlock {
    pub fn from_token(token: &Token) -> Option<Self> {
        if let Token::Array(arr) = token {
            if let Token::Tuple(tuple) = arr.last()? {
                let block_number = if let Token::Uint(block_number) = tuple[0].clone() {
                    block_number
                } else {
                    return None;
                };
                let l1_request_count = if let Token::Uint(l1_request_count) = tuple[1].clone() {
                    l1_request_count
                } else {
                    return None;
                };
                let pending_rollup_tx_hash =
                    if let Token::FixedBytes(pending_rollup_tx_hash) = tuple[2].clone() {
                        pending_rollup_tx_hash
                    } else {
                        return None;
                    };
                let commitment = if let Token::FixedBytes(commitment) = tuple[3].clone() {
                    commitment
                } else {
                    return None;
                };
                let state_root = if let Token::FixedBytes(state_root) = tuple[4].clone() {
                    state_root
                } else {
                    return None;
                };
                let timestamp = if let Token::Uint(timestamp) = tuple[5].clone() {
                    timestamp
                } else {
                    return None;
                };
                Some(ExecutedBlock {
                    block_number,
                    l1_request_count,
                    pending_rollup_tx_hash,
                    commitment,
                    state_root,
                    timestamp,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}
impl std::fmt::Display for ExecutedBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format the struct fields into a JSON-like string
        write!(
            f,
            r#"{{
    "block_number": {},
    "l1_request_count": {},
    "pending_rollup_tx_hash": "0x{}",
    "commitment": "0x{}",
    "state_root": "0x{}",
    "timestamp": {}
}}"#,
            self.block_number,
            self.l1_request_count,
            hex::encode(&self.pending_rollup_tx_hash),
            hex::encode(&self.commitment),
            hex::encode(&self.state_root),
            self.timestamp
        )
    }
}
