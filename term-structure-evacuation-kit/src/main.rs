extern crate term_structure_evacuation_kit;
use clap::{App, Arg, SubCommand};
use term_structure_evacuation_kit::{get_consume_data, get_evacu_prf, query_funds, update_state, Config};

fn main() {
    let matches = App::new("ts-evacu")
        .version("1.0")
        .author("Term Structure Lib. @tkspirng.com")
        .about("Term Structure Evacuation Kit")
        .subcommand(
            SubCommand::with_name("update_state")
                .about("Updates the state")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .required(true)
                        .help("Sets a custom config file"),
                )
                .arg(
                    Arg::with_name("end_block_id")
                        .short("e")
                        .long("endblock")
                        .takes_value(true)
                        .help("End block id"),
                ),
        )
        .subcommand(
            SubCommand::with_name("export")
                .about("Export the input files for the evacuation zk proof")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .required(true)
                        .help("Sets a custom config file"),
                )
                .arg(
                    Arg::with_name("acc_id")
                        .short("a")
                        .long("accid")
                        .takes_value(true)
                        .required(true)
                        .help("Account ID"),
                )
                .arg(
                    Arg::with_name("token_id")
                        .short("t")
                        .long("tokenid")
                        .takes_value(true)
                        .required(true)
                        .help("Token ID"),
                ),
        )
        .subcommand(
            SubCommand::with_name("query")
                .about("Query the balance of a specific account for a specified asset")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .required(true)
                        .help("Sets a custom config file"),
                )
                .arg(
                    Arg::with_name("acc_id")
                        .short("a")
                        .long("accid")
                        .takes_value(true)
                        .required(true)
                        .help("Account ID"),
                )
                .arg(
                    Arg::with_name("token_id")
                        .short("t")
                        .long("tokenid")
                        .takes_value(true)
                        .required(true)
                        .help("Token ID"),
                ),
        )
        .subcommand(
            SubCommand::with_name("consume")
                .about("Exports the data required to consume L1 requests in the smart contract")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .required(true)
                        .help("Sets a custom config file"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("update_state") {
        let config_path = matches.value_of("config").unwrap_or("default_path");
        let config = match load_config(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("[Error] Failed to load config: {}", e);
                return;
            }
        };

        let end_block_id = match matches.value_of("end_block_id") {
            Some(s) => match s.parse::<usize>() {
                Ok(num) => Some(num),
                Err(_) => {
                    eprintln!("[Error] Invalid end block id");
                    return;
                }
            },
            None => None,
        };

        if let Err(e) = update_state(config, end_block_id) {
            eprintln!("[Error] Failed to update state: {}", e);
            return;
        }
    }

    if let Some(matches) = matches.subcommand_matches("export") {
        let config_path = matches.value_of("config").unwrap_or("default_path");
        let config = match load_config(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("[Error] Failed to load config: {}", e);
                return;
            }
        };

        let acc_id = match matches.value_of("acc_id") {
            Some(num) => match num.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("[Error] Invalid account id");
                    return;
                }
            },
            None => {
                eprintln!("unreachable");
                return;
            }
        };

        let token_id = match matches.value_of("token_id") {
            Some(num) => match num.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("[Error] Invalid token id");
                    return;
                }
            },
            None => {
                eprintln!("unreachable");
                return;
            }
        };

        match get_evacu_prf(config, acc_id, token_id) {
            Ok(evacu_proof) => match serde_json::to_string(&evacu_proof) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("[Error] Failed to serialize evacuation proof: {}", e),
            },
            Err(e) => eprintln!("[Error] Failed to get evacuation proof: {}", e),
        }
    }

    if let Some(matches) = matches.subcommand_matches("query") {
        let config_path = matches.value_of("config").unwrap_or("default_path");
        let config = match load_config(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("[Error] Failed to load config: {}", e);
                return;
            }
        };

        let acc_id = match matches.value_of("acc_id") {
            Some(num) => match num.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("[Error] Invalid account id");
                    return;
                }
            },
            None => {
                eprintln!("unreachable");
                return;
            }
        };

        let token_id = match matches.value_of("token_id") {
            Some(num) => match num.parse::<usize>() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("[Error] Invalid token id");
                    return;
                }
            },
            None => {
                eprintln!("unreachable");
                return;
            }
        };

        match query_funds(config, acc_id, token_id) {
            Ok(funds) => match serde_json::to_string(&funds) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("[Error] Failed to serialize funds: {}", e),
            },
            Err(e) => eprintln!("[Error] Failed to query funds: {}", e),
        }
    }

    if let Some(matches) = matches.subcommand_matches("consume") {
        let config_path = matches.value_of("config").unwrap_or("default_path");
        let config = match load_config(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("[Error] Failed to load config: {}", e);
                return;
            }
        };

        match get_consume_data(config) {
            Ok(consume_data) => match serde_json::to_string(&consume_data) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("[Error] Failed to export the data: {}", e),
            },
            Err(e) => eprintln!("[Error] Failed to export the data: {}", e),
        }
    }
}

fn load_config(path: &str) -> Result<Config, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| e.to_string())
}
