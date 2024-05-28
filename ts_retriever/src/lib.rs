mod block;

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

async fn get_latest_executed_l2_block_id(
    api_link: &str,
    api_key: &str,
    ts_addr: &str,
    blocks_per_batch: usize,
) -> Result<u32, String> {
    // Set up web3 connection
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link).map_err(|e| e.to_string())?;
    let web3 = web3::Web3::new(http);

    // Contract address and event signature
    let contract_address: H160 = ts_addr
        .parse()
        .map_err(|_| "Loading contract address failed")?;
    let event_signature = H256::from_slice(&keccak256("BlockExecution(uint32)"));

    let latest_block = web3.eth().block_number().await.map_err(|e| e.to_string())?;

    let mut end_block = latest_block;
    let mut start_block = if latest_block > blocks_per_batch.into() {
        latest_block - blocks_per_batch
    } else {
        0.into()
    };
    while start_block != 0.into() {
        let filter = FilterBuilder::default()
            .address(vec![contract_address])
            .from_block(BlockNumber::Number(start_block.into()))
            .to_block(BlockNumber::Number(end_block.into()))
            .topics(Some(vec![event_signature]), None, None, None)
            .build();

        let logs = web3.eth().logs(filter).await.map_err(|e| e.to_string())?;

        if logs.is_empty() {
            end_block = start_block - 1;
            start_block = if start_block > blocks_per_batch.into() {
                start_block - blocks_per_batch
            } else {
                0.into()
            };
            continue;
        } else {
            let last_log = logs.last().unwrap();

            if let Some(block_number_data) = last_log.topics.get(1) {
                let l2_block_number =
                    ethabi::decode(&[ethabi::ParamType::Uint(32)], &block_number_data.0)
                        .map_err(|e| e.to_string())?;
                if let Token::Uint(value) = l2_block_number[0] {
                    return Ok(value.low_u32());
                } else {
                    return Err("Loading Latest Executed L2 block number failed".to_string());
                }
            }
        }
    }
    Err("No Executed Block".to_string())
}

pub async fn retrieve(
    abi_json_str: &str,
    api_link: &str,
    api_key: &str,
    ts_contract_addr: &str,
    start_block_id: usize,
    end_block_id: Option<usize>,
    max_parallel_calls: usize,
    mut do_block: impl FnMut(u64, Block) -> Result<(), String>,
) -> Result<(), String> {
    let mut futures = FuturesOrdered::new();

    // Parse ABI string to ethabi::Contract
    let contract = Contract::load(abi_json_str.as_bytes()).map_err(|e| e.to_string())?;

    let latest_executed_l2_block_id =
        get_latest_executed_l2_block_id(api_link, api_key, ts_contract_addr, 100).await?;

    // Set up web3 connection
    let link = format!("{}{}", api_link, api_key);
    let http = web3::transports::Http::new(&link).map_err(|e| e.to_string())?;
    let web3 = web3::Web3::new(http);

    // Contract address and event signature
    let contract_address: H160 = ts_contract_addr
        .parse()
        .map_err(|_| "Loading contract address failed")?;
    let event_signature = H256::from_slice(&keccak256("BlockCommit(uint32,bytes32)"));

    // Create a filter for a range of blocks
    let filters = vec![FilterBuilder::default()
        .address(vec![contract_address])
        .topics(Some(vec![event_signature]), None, None, None)
        .from_block(BlockNumber::from(start_block_id)) // Start block
        .to_block(match end_block_id {
            Some(end_block_id) => BlockNumber::from(end_block_id), // End block
            None => BlockNumber::Latest,
        }) // End block
        .build()];

    for filter in filters {
        // Query the filter for logs
        let logs = web3.eth().logs(filter).await.map_err(|e| e.to_string())?;

        let mut prev_tx_hash: Option<H256> = None;
        for (idx, log) in logs.iter().enumerate() {
            // Process each log
            let block_id: BlockId = BlockId::Number(BlockNumber::Number(
                log.block_number.ok_or("Loading block number failed")?,
            ));
            let tx_id: TransactionId = TransactionId::Block(
                block_id,
                log.transaction_index.ok_or("Loading tx id failed")?,
            );
            let tx_hash: H256 = log.transaction_hash.ok_or("Loading tx hash failed")?;
            if prev_tx_hash == Some(tx_hash) {
                continue;
            }
            prev_tx_hash = Some(tx_hash);

            if let Some(block_number_data) = log.topics.get(1) {
                let l2_block_number =
                    ethabi::decode(&[ethabi::ParamType::Uint(32)], &block_number_data.0)
                        .map_err(|e| e.to_string())?;
                if let Token::Uint(value) = l2_block_number[0] {
                    if value.low_u32() <= latest_executed_l2_block_id {
                        futures.push_back(async move {
                            // Set up web3 connection
                            let link = format!("{}{}", api_link, api_key);
                            let http = web3::transports::Http::new(&link)?;
                            let web3 = web3::Web3::new(http);
                            web3.eth().transaction(tx_id).await
                        });
                    }
                } else {
                    return Err("Loading Committed L2 block number failed".to_string());
                }
            }
            if idx % max_parallel_calls == max_parallel_calls - 1 || idx == logs.len() - 1 {
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
                                    let block =
                                        Block::from_token(block).ok_or("Loading block failed")?;
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
                            println!("Tx not found");
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
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
        println!("Current block: {:?}", current_block);

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
