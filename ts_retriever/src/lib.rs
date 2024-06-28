mod block;

use std::sync::Arc;

pub use block::Block;
use web3::{
    ethabi::{Contract, Token},
    futures::{stream::FuturesOrdered, StreamExt},
    types::{BlockId, BlockNumber, CallRequest, FilterBuilder, Log, TransactionId, H160, H256},
};

fn keccak256(input: &str) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}

struct EventTracer {
    logs: Vec<Log>,
    current_idx: usize,
    last_l1_block_id: u64,
    end_block_id: u64,
    blocks_per_batch: usize,
    api_link: String,
    api_key: String,
    ts_addr: String,
    event_signature: H256,
}
impl EventTracer {
    fn new(
        api_link: String,
        api_key: String,
        ts_addr: String,
        start_block_id: u64,
        end_block_id: u64,
        blocks_per_batch: usize,
        event_signature_string: String,
    ) -> Self {
        Self {
            logs: Vec::new(),
            current_idx: 0,
            last_l1_block_id: start_block_id - 1,
            end_block_id,
            blocks_per_batch,
            api_link,
            api_key,
            ts_addr,
            event_signature: H256::from_slice(&keccak256(&event_signature_string)),
        }
    }
    async fn pop(&mut self) -> Result<Option<(TransactionId, u32)>, String> {
        while self.current_idx == self.logs.len() {
            // Set up web3 connection
            let link = format!("{}{}", self.api_link, self.api_key);
            let http = web3::transports::Http::new(&link).map_err(|e| e.to_string())?;
            let web3 = web3::Web3::new(http);

            // Contract address and event signature
            let contract_address: H160 = self
                .ts_addr
                .parse()
                .map_err(|_| "Loading contract address failed")?;
            let event_signature = self.event_signature;

            let start_block_id: web3::types::U64 = (self.last_l1_block_id + 1).into();
            let end_block_id = if start_block_id + self.blocks_per_batch > self.end_block_id.into()
            {
                self.end_block_id.into()
            } else {
                start_block_id + self.blocks_per_batch
            };

            if start_block_id >= end_block_id {
                return Ok(None);
            }

            let filter = FilterBuilder::default()
                .address(vec![contract_address])
                .from_block(BlockNumber::Number(start_block_id.into()))
                .to_block(BlockNumber::Number(end_block_id.into()))
                .topics(Some(vec![event_signature]), None, None, None)
                .build();
            self.logs = web3.eth().logs(filter).await.map_err(|e| e.to_string())?;

            self.last_l1_block_id = end_block_id.as_u64();
            self.current_idx = 0;
        }

        let log = &self.logs[self.current_idx];
        let block_id: BlockId = BlockId::Number(BlockNumber::Number(
            log.block_number.ok_or("Loading block number failed")?,
        ));
        let tx_id: TransactionId = TransactionId::Block(
            block_id,
            log.transaction_index.ok_or("Loading tx id failed")?,
        );

        let l2_block_id = if let Some(block_number_data) = log.topics.get(1) {
            let l2_block_number =
                ethabi::decode(&[ethabi::ParamType::Uint(32)], &block_number_data.0)
                    .map_err(|e| e.to_string())?;
            if let Token::Uint(value) = l2_block_number[0] {
                Ok(value.low_u32())
            } else {
                Err("Loading Committed L2 block number failed".to_string())
            }
        } else {
            return Err("Loading Committed L2 block number failed".to_string());
        }?;
        self.current_idx += 1;
        Ok(Some((tx_id, l2_block_id)))
    }
}

pub async fn retrieve(
    abi_json_str: &str,
    api_link: &str,
    api_key: &str,
    ts_contract_addr: &str,
    start_block_id: usize,
    end_block_id: Option<usize>,
    max_parallel_calls: usize,
    filter_batch_size: usize,
    mut do_block: impl FnMut(u64, Block) -> Result<(), String>,
) -> Result<(), String> {
    let blocks_per_batch = filter_batch_size;
    let mut futures = FuturesOrdered::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel_calls));

    // Parse ABI string to ethabi::Contract
    let contract = Contract::load(abi_json_str.as_bytes()).map_err(|e| e.to_string())?;

    // Set up web3 connection
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link).map_err(|e| e.to_string())?;
    let web3 = web3::Web3::new(http);
    let end_block_id: u64 = match end_block_id {
        Some(end_block_id) => end_block_id as u64,
        None => web3
            .eth()
            .block_number()
            .await
            .map_err(|e| e.to_string())?
            .as_u64(),
    };

    let mut executed_envent_tracer = EventTracer::new(
        api_link.to_string(),
        api_key.to_string(),
        ts_contract_addr.to_string(),
        start_block_id as u64,
        end_block_id,
        blocks_per_batch,
        "BlockExecution(uint32)".to_string(),
    );
    let mut committed_event_tracer = EventTracer::new(
        api_link.to_string(),
        api_key.to_string(),
        ts_contract_addr.to_string(),
        start_block_id as u64,
        end_block_id,
        blocks_per_batch,
        "BlockCommit(uint32,bytes32)".to_string(),
    );

    let mut current_executed_block_info = executed_envent_tracer.pop().await?;
    let mut current_committed_block_info = committed_event_tracer.pop().await?;

    fn slt(
        a: &Option<(TransactionId, u32)>,
        b: &Option<(TransactionId, u32)>,
    ) -> Result<bool, String> {
        match (a, b) {
            (None, None) => Ok(false),
            (Some(_), None) => Ok(true),
            (None, Some(_)) => Ok(false),
            (
                Some((TransactionId::Block(a_block_id, a_tx_index), _)),
                Some((TransactionId::Block(b_block_id, b_tx_index), _)),
            ) => {
                if a_block_id == b_block_id {
                    Ok(a_tx_index < b_tx_index)
                } else {
                    if let (
                        BlockId::Number(BlockNumber::Number(a_block_number)),
                        BlockId::Number(BlockNumber::Number(b_block_number)),
                    ) = (a_block_id, b_block_id)
                    {
                        Ok(a_block_number < b_block_number)
                    } else {
                        Err("Invalid BlockId".to_string())
                    }
                }
            }
            _ => Err("Invalid TransactionId".to_string()),
        }
    }

    let mut queue = std::collections::VecDeque::<(TransactionId, u32)>::new();
    while current_executed_block_info.is_some() {
        if slt(&current_executed_block_info, &current_committed_block_info)? {
            if let Some((_, executed_l2_block_id)) = &current_executed_block_info {
                if queue.len() != 0 {
                    if let Some((tx_id, l2_block_id)) = queue.get(0) {
                        if executed_l2_block_id.clone() > l2_block_id.clone() {
                            return Err("Loading block info failed - 1".to_string());
                        } else if executed_l2_block_id.clone() == l2_block_id.clone() {
                            let semaphore_clone = semaphore.clone();
                            let tx_id = tx_id.clone();
                            futures.push_back(async move {
                                // Set up web3 connection
                                let link = format!("{}{}", api_link, api_key);
                                let http = web3::transports::Http::new(&link)
                                    .map_err(|e| e.to_string())?;
                                let web3 = web3::Web3::new(http);
                                let permit = semaphore_clone.acquire_owned().await.unwrap();
                                let result = web3.eth().transaction(tx_id).await;
                                drop(permit);
                                result
                            });
                            queue.pop_front();
                        }
                    }
                }
            } else {
                return Err("Loading block info failed - 2".to_string());
            }
            current_executed_block_info = executed_envent_tracer.pop().await?;
        } else {
            if let Some((committed_tx_id, committed_l2_block_id)) = &current_committed_block_info {
                if queue.len() == 0 {
                    queue.push_back((committed_tx_id.clone(), committed_l2_block_id.clone()));
                } else {
                    match queue.get(queue.len() - 1) {
                        None => {
                            unreachable!()
                        }
                        Some((_, l2_block_id)) => {
                            if committed_l2_block_id > l2_block_id {
                                queue.push_back((
                                    committed_tx_id.clone(),
                                    committed_l2_block_id.clone(),
                                ));
                            } else {
                                let mut i = queue.len();
                                let mut is_changed = false;
                                while i > 0 {
                                    if let Some((_, l2_block_id)) = queue.get(i - 1) {
                                        if committed_l2_block_id == l2_block_id {
                                            *queue.get_mut(i - 1).unwrap() = (
                                                committed_tx_id.clone(),
                                                committed_l2_block_id.clone(),
                                            );
                                            is_changed = true;
                                            break;
                                        } else if committed_l2_block_id > l2_block_id {
                                            return Err("Loading block info failed - 3".to_string());
                                        }
                                    } else {
                                        unreachable!()
                                    }
                                    i -= 1;
                                }
                                if !is_changed {
                                    return Err("Loading block info failed - 4".to_string());
                                }
                            }
                        }
                    }
                }
                current_committed_block_info = committed_event_tracer.pop().await?;
            } else {
                return Err("Loading block info failed - 5".to_string());
            }
        }
    }
    while let Some(result) = futures.next().await {
        match result {
            Ok(Some(tx)) => {
                let calldata: Vec<u8> = tx.input.0;
                let input = &calldata[4..];
                let function = contract
                    .function("commitBlocks")
                    .map_err(|e| e.to_string())?;
                let tokens = function.decode_input(input).map_err(|e| e.to_string())?;
                if let Some(Token::Array(new_blocks)) = tokens.get(1) {
                    for block in new_blocks {
                        let block = Block::from_token(block).ok_or("Loading block failed")?;
                        do_block(
                            tx.block_number
                                .ok_or("Loading block number failed")?
                                .as_u64(),
                            block,
                        )?;
                    }
                }
            }
            Ok(None) => {
                return Err("Tx not found".to_string());
            }
            Err(e) => {
                return Err(format!("Error: {:?}", e));
            }
        }
    }
    Ok(())
}

pub async fn retrieve_consume_data(
    api_link: &str,
    api_key: &str,
    ts_contract_addr: &str,
    remaining_l1_req_count: usize,
) -> Result<Vec<String>, String> {
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link)
        .map_err(|e| e.to_string())
        .unwrap();
    let web3 = web3::Web3::new(http);

    let contract_address: H160 = ts_contract_addr
        .parse()
        .map_err(|_| "Loading contract address failed")
        .unwrap();

    let deposit_event_signature = H256::from_slice(&keccak256(
        "Deposit(address,address,uint32,address,uint16,uint128)",
    ));
    let register_event_signature = H256::from_slice(&keccak256(
        "Registration(address,uint32,uint256,uint256,bytes20)",
    ));
    let force_withdraw_event_signature =
        H256::from_slice(&keccak256("ForceWithdrawal(address,uint32,address,uint16)"));

    let latest_block = web3.eth().block_number().await.map_err(|e| e.to_string())?;

    let mut logs: Vec<Log> = Vec::new();
    let mut current_block = latest_block;
    while logs.len() < remaining_l1_req_count {
        let from_block: BlockNumber = if current_block > 1000.into() {
            (current_block - web3::types::U64::from(1000)).into()
        } else {
            BlockNumber::Earliest
        };

        let filter = FilterBuilder::default()
            .address(vec![contract_address])
            .topics(
                Some(vec![
                    deposit_event_signature,
                    register_event_signature,
                    force_withdraw_event_signature,
                ]),
                None,
                None,
                None,
            )
            .from_block(from_block)
            .to_block(current_block.into())
            .build();

        let new_logs = web3.eth().logs(filter).await.map_err(|e| e.to_string())?;
        logs.extend(new_logs);

        if current_block > 2000.into() {
            current_block = current_block - web3::types::U64::from(2001);
        } else {
            break;
        }
    }

    logs.sort_by(|a, b| b.block_number.cmp(&a.block_number));
    let latest_logs = logs
        .into_iter()
        .take(remaining_l1_req_count)
        .collect::<Vec<Log>>();

    let result = latest_logs
        .iter()
        .map(|log| hex::encode(&log.data.0))
        .collect::<Vec<String>>();

    Ok(result)
}

pub async fn is_evacuation_mod(
    api_link: &str,
    api_key: &str,
    ts_contract_addr: &str,
) -> Result<bool, String> {
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link)
        .map_err(|e| e.to_string())
        .unwrap();
    let web3 = web3::Web3::new(http);

    let contract_address: H160 = ts_contract_addr
        .parse()
        .map_err(|_| "Loading contract address failed")
        .unwrap();

    let call_request = CallRequest {
        from: None,
        to: Some(contract_address),
        gas: None,
        gas_price: None,
        value: None,
        data: Some(keccak256("isEvacuMode()")[..4].to_vec().into()),
        transaction_type: None,
        access_list: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = web3
        .eth()
        .call(call_request, None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.0[0] == 1)
}

pub async fn get_remaining_l1_req_count(
    api_link: &str,
    api_key: &str,
    ts_contract_addr: &str,
) -> Result<usize, String> {
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link)
        .map_err(|e| e.to_string())
        .unwrap();
    let web3 = web3::Web3::new(http);

    let contract_address: H160 = ts_contract_addr
        .parse()
        .map_err(|_| "Loading contract address failed")
        .unwrap();

    let call_request = CallRequest {
        from: None,
        to: Some(contract_address),
        gas: None,
        gas_price: None,
        value: None,
        data: Some(keccak256("getL1RequestNum()")[..4].to_vec().into()),
        transaction_type: None,
        access_list: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let result = web3
        .eth()
        .call(call_request, None)
        .await
        .map_err(|e| e.to_string())?;

    let committed_l1_req_count: u64 = result.0[..8]
        .iter()
        .rev()
        .fold(0, |acc, &x| acc * 256 + x as u64);
    let executed_l1_req_count: u64 = result.0[8..16]
        .iter()
        .rev()
        .fold(0, |acc, &x| acc * 256 + x as u64);

    Ok((committed_l1_req_count - executed_l1_req_count) as usize)
}
