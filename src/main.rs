extern crate sputnikvm;
extern crate sputnikvm_stateful;
extern crate secp256k1;
extern crate rand;
extern crate sha3;
extern crate blockchain;
extern crate bigint;
extern crate rlp;
extern crate bloom;
extern crate block;
extern crate trie;
extern crate hexutil;
#[macro_use]
extern crate lazy_static;
extern crate jsonrpc_core;
extern crate jsonrpc_http_server;
#[macro_use]
extern crate jsonrpc_macros;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;

#[cfg(feature = "frontend")]
extern crate hyper;

mod error;
mod miner;
mod rpc;

#[cfg(feature = "frontend")]
mod assets;

use miner::MinerState;
use rand::os::OsRng;
use secp256k1::key::{PublicKey, SecretKey};
use secp256k1::SECP256K1;
use bigint::U256;
use hexutil::*;
use std::thread;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use sputnikvm::MainnetEIP160Patch;

fn main() {
    env_logger::init();
    let mut rng = OsRng::new().unwrap();

    let matches = clap_app!(
        svmdev =>
            (version: "0.1")
            (author: "Ethereum Classic Contributors")
            (about: "SputnikVM Development Environment, a replacement for ethereumjs-testrpc.")
            (@arg PRIVATE_KEY: -k --private +takes_value "Private key for the account to be generated, if not provided, a random private key will be generated.")
            (@arg BALANCE: -b --balance +takes_value "Balance in Wei for the account to be generated, default is 0x10000000000000000000000000000.")
            (@arg LISTEN: -l --listen +takes_value "Listen address and port for the RPC, e.g. 127.0.0.1:8545.")
            (@arg ACCOUNTS: -a --accounts +takes_value "Additional accounts to be generated, default to 9.")
    ).get_matches();

    let secret_key = match matches.value_of("PRIVATE_KEY") {
        Some(val) => SecretKey::from_slice(&SECP256K1, &read_hex(val).unwrap()).unwrap(),
        None => SecretKey::new(&SECP256K1, &mut rng),
    };

    let balance = {
        let s = matches.value_of("BALANCE").unwrap_or("0x10000000000000000000000000000");
        if s.starts_with("0x") {
            U256::from_str(s).unwrap()
        } else {
            U256::from_dec_str(s).unwrap()
        }
    };
    let accounts_len: usize = match matches.value_of("ACCOUNTS") {
        Some(val) => val.parse().unwrap(),
        None => 9,
    };

    let mut genesis = Vec::new();
    genesis.push((secret_key, balance));

    for _ in 0..accounts_len {
        genesis.push((SecretKey::new(&SECP256K1, &mut rng), balance));
    }

    let (sender, receiver) = channel::<bool>();

    let state = miner::make_state::<MainnetEIP160Patch>(genesis);

    let miner_arc = Arc::new(Mutex::new(state));
    let rpc_arc = miner_arc.clone();

    thread::spawn(move || {
        miner::mine_loop::<MainnetEIP160Patch>(miner_arc, receiver);
    });

    #[cfg(feature = "frontend")]
    {
        thread::spawn(move || {
            use hyper::Server;
            use hyper::server::Request;
            use hyper::server::Response;
            use hyper::uri::RequestUri::AbsolutePath;

            fn handle_index(req: Request, res: Response) {
                match req.uri {
                    AbsolutePath(ref path) => {
                        println!("GET {}", &path);
                        if &path[..] == "/" {
                            res.send(&assets::__index_html).unwrap();
                        } else {
                            match assets::get(&format!(".{}", path)) {
                                Ok(val) => {
                                    res.send(val).unwrap();
                                },
                                Err(e) => {
                                    res.send(e.as_bytes()).unwrap();
                                },
                            }
                        }
                    },
                    _ => {
                        return;
                    }
                }
            }

            Server::http("127.0.0.1:8380").unwrap().handle(handle_index).unwrap();
        });
    }

    rpc::rpc_loop::<MainnetEIP160Patch>(
        rpc_arc,
        &matches.value_of("LISTEN").unwrap_or("127.0.0.1:8545").parse().unwrap(),
        sender);
}
