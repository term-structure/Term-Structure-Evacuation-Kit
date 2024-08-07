use instance::TsFile;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use ts_merkle_tree::{MerkleTree, MerkleTreeWithLeaves};
use ts_retriever::{
    get_remaining_l1_req_count, retrieve, retrieve_consume_data, retrieve_last_excuted_block,
};
use ts_state::{constants::TX_COUNT_PER_BLOCK, Array, Value};
use ts_tx::{parser::Schema, Tx};

pub mod instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    ts_filename: String,
    ts_contract_addr: String,
    api_key: String,
    api_link: String,
    l2_genesis_l1_anchor_id: u64,
    max_parallel_calls: u64,
    filter_batch_size: u64,
}

pub fn update_state(cfg: Config, end_block_id: Option<usize>) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    TsFile::perform_with_file(
        cfg.ts_filename.as_str(),
        Some(cfg.l2_genesis_l1_anchor_id),
        |ts_file| {
            let sechma: Schema = serde_json::from_str(&include_str!("../ZkTrueUp_Tx_Schema.json"))
                .map_err(|e| e.to_string())?;
            let start_block_id = ts_file.latest_l1_block_id + 1;

            println!("[ts_file loaded]");
            println!("    latest_l1_block_id: {}", ts_file.latest_l1_block_id);
            println!("    block_count: {}", ts_file.block_count);
            println!("# ===================== #");

            rt.block_on(retrieve(
                include_str!("../ZkTrueUp_IRollupFacet_ABI.json"),
                cfg.api_link.as_str(),
                cfg.api_key.as_str(),
                cfg.ts_contract_addr.as_str(),
                start_block_id as usize,
                end_block_id,
                cfg.max_parallel_calls as usize,
                cfg.filter_batch_size as usize,
                |l1_anchor_id, block| {
                    let mut state = ts_file.to_state()?;
                    let block_id = ts_file.block_count as usize;
                    if block.block_number.as_usize() < block_id {
                        return Ok(()); // skip
                    }
                    println!("    processing block {} ...", block_id);
                    // println!("public_data: {}", hex::encode(&block.public_data[..64]));
                    let tmp = block.public_data.clone();
                    let mut tmp = tmp.as_slice();
                    let mut tx_id_offset = 0;
                    while tmp.len() != 0 {
                        let res = match sechma.parse(&mut tmp) {
                            Ok(res) => {
                                // println!("    tx_id_offset {:>3}: {:#?}", tx_id_offset, res);
                                res
                            }
                            Err(e) => {
                                println!("# ===================== #");
                                println!("    block: {}", block_id);
                                println!("    tmp[0]: {:#?}", tmp[0]);
                                return Err(e.to_string());
                            }
                        };
                        if let Tx::TxNoop(_) = res {
                            break;
                        }
                        match state.push_tx((block_id - 1) * TX_COUNT_PER_BLOCK + tx_id_offset, res)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                println!("# ===================== #");
                                println!("    block: {}", block_id);
                                println!("    res: {:#?}", res);
                                return Err(e.to_string());
                            }
                        }
                        tx_id_offset += 1;
                    }
                    for j in 0..tx_id_offset {
                        match state.update((block_id - 1) * TX_COUNT_PER_BLOCK + j) {
                            Err(e) => {
                                println!("# ===================== #");
                                println!("    block: {}", block_id);
                                println!("    tx_id: {}", (block_id - 1) * TX_COUNT_PER_BLOCK + j);
                                println!(
                                    "res: {:#?}",
                                    state
                                        .txs
                                        .get((block_id - 1) * TX_COUNT_PER_BLOCK + j)?
                                        .raw_tx
                                );
                                return Err(e.to_string());
                            }
                            Ok(_) => {
                                // println!(
                                //     "    exec tx {:>3}: {}",
                                //     (block_id - 1) * TX_COUNT_PER_BLOCK + j,
                                //     ts_merkle_tree::MerkleTree::get_root(&state.accounts)?
                                // );
                            }
                        }
                    }
                    let new_ts_root: ark_bn254::Fr =
                        BigUint::from_bytes_be(&block.new_ts_root).into();
                    let new_state_root: ark_bn254::Fr =
                        BigUint::from_bytes_be(&block.new_state_root).into();
                    state.set_ts_root(new_ts_root)?;
                    if state.get_root()? != new_state_root {
                        println!("# ===================== #");
                        println!("    block: {}", block_id);
                        println!("    state.get_root()?: {}", state.get_root()?);
                        println!("    new_state_root: {}", new_state_root);
                        return Err("state root mismatch".to_string());
                    }
                    ts_file.block_count += 1;
                    ts_file.latest_l1_block_id = l1_anchor_id as u64;
                    ts_file.sync()?;
                    Ok(())
                },
            ))
            .map_err(|e| e.to_string())?;
            Ok(())
        },
    )
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvacuProof {
    #[serde(rename = "currentTime")]
    current_time: String,
    #[serde(rename = "stateRoot")]
    state_root: String,
    #[serde(rename = "tsRoot")]
    ts_root: String,
    #[serde(rename = "accRoot")]
    acc_root: String,
    #[serde(rename = "accId")]
    acc_id: String,
    #[serde(rename = "nonce")]
    nonce: String,
    #[serde(rename = "tsAddr")]
    ts_addr: String,
    #[serde(rename = "tokenRoot")]
    token_root: String,
    #[serde(rename = "tokenId")]
    token_id: String,
    #[serde(rename = "avlAmt")]
    avl_amt: String,
    #[serde(rename = "lockedAmt")]
    locked_amt: String,
    #[serde(rename = "accMkPrf")]
    acc_mk_prf: Vec<String>,
    #[serde(rename = "tokenMkPrf")]
    token_mk_prf: Vec<String>,
}
pub fn get_evacu_prf(cfg: Config, acc_id: usize, token_id: usize) -> Result<EvacuProof, String> {
    let mut evacu_proof = EvacuProof::default();
    TsFile::perform_with_file(
        cfg.ts_filename.as_str(),
        Some(cfg.l2_genesis_l1_anchor_id),
        |ts_file| {
            let state = ts_file.to_state()?;
            let acc_prf = state.accounts.verify_leaf(acc_id)?;
            let account = state.accounts.leaf_at(acc_id)?;
            let token_prf = account.tokens.verify_leaf(token_id)?;

            evacu_proof.state_root = state.get_root()?.to_string();
            evacu_proof.ts_root = state.ts_root.get()?.to_string();
            evacu_proof.acc_root = state.accounts.get_root()?.to_string();
            evacu_proof.acc_id = acc_id.to_string();
            evacu_proof.nonce = account.nonce.to_string();
            evacu_proof.ts_addr = account.l2_addr.to_string();
            evacu_proof.token_root = account.tokens.get_root()?.to_string();
            evacu_proof.token_id = token_id.to_string();
            evacu_proof.avl_amt = token_prf.leaf.avl_amt.to_string();
            evacu_proof.locked_amt = token_prf.leaf.locked_amt.to_string();
            evacu_proof.acc_mk_prf = acc_prf
                .merkle_prf
                .proof
                .iter()
                .map(|x| x.to_string())
                .collect();
            evacu_proof.token_mk_prf = token_prf
                .merkle_prf
                .proof
                .iter()
                .map(|x| x.to_string())
                .collect();
            Ok(())
        },
    )?;
    evacu_proof.current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        .to_string();
    Ok(evacu_proof)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Funds {
    acc_id: usize,
    token_id: usize,
    avl_amt: String,
    locked_amt: String,
}
pub fn query_funds(cfg: Config, acc_id: usize, token_id: usize) -> Result<Funds, String> {
    let mut funds = Funds::default();
    TsFile::perform_with_file(
        cfg.ts_filename.as_str(),
        Some(cfg.l2_genesis_l1_anchor_id),
        |ts_file| {
            let state = ts_file.to_state()?;
            let account = state.accounts.leaf_at(acc_id)?;
            let token = account.tokens.leaf_at(token_id)?;
            funds.acc_id = acc_id;
            funds.token_id = token_id;
            funds.avl_amt = token.avl_amt.to_string();
            funds.locked_amt = token.locked_amt.to_string();
            Ok(())
        },
    )?;
    Ok(funds)
}

pub fn get_consume_data(cfg: Config) -> Result<Vec<String>, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let remaining_l1_req_count = rt
        .block_on(get_remaining_l1_req_count(
            cfg.api_link.as_str(),
            cfg.api_key.as_str(),
            cfg.ts_contract_addr.as_str(),
        ))
        .map_err(|e| e.to_string())?;

    let consume_data = rt
        .block_on(retrieve_consume_data(
            cfg.api_link.as_str(),
            cfg.api_key.as_str(),
            cfg.ts_contract_addr.as_str(),
            remaining_l1_req_count,
            cfg.filter_batch_size as usize,
        ))
        .map_err(|e| e.to_string())?;

    Ok(consume_data)
}

pub fn get_last_excuted_block(cfg: Config) -> Result<String, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    let data = rt
        .block_on(retrieve_last_excuted_block(
            cfg.api_link.as_str(),
            cfg.api_key.as_str(),
            cfg.ts_contract_addr.as_str(),
            cfg.filter_batch_size as usize,
        ))
        .map_err(|e| e.to_string())?;

    Ok(data)
}
