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

mod error;
mod miner;
mod rpc;

use rand::os::OsRng;
use secp256k1::key::{PublicKey, SecretKey};
use secp256k1::SECP256K1;
use bigint::U256;
use hexutil::*;
use std::thread;
use std::str::FromStr;

fn main() {
    env_logger::init();
    let mut rng = OsRng::new().unwrap();

    let matches = clap_app!(
        svmdev =>
            (version: "0.1")
            (author: "Ethereum Classic Contributors")
            (about: "SputnikVM Development Environment, a replacement for ethereumjs-testrpc.")
            (@arg PRIVATE_KEY: -p --private-key +takes_value "Private key for the account to be generated, if not provided, a random private key will be generated.")
            (@arg BALANCE: -b --balance +takes_value "Balance in Wei for the account to be generated, default is 0x10000000000000000000000000000.")
            (@arg LISTEN: -l --listen +takes_value "Listen address and port for the RPC, e.g. 127.0.0.1:8545")
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

    thread::spawn(move || {
        miner::mine_loop(secret_key, balance);
    });

    rpc::rpc_loop(&matches.value_of("LISTEN").unwrap_or("127.0.0.1:8545").parse().unwrap());
}
