extern crate sputnikvm;
extern crate secp256k1;
extern crate rand;
extern crate sha3;
extern crate blockchain;
extern crate bigint;
extern crate rlp;
extern crate bloom;
extern crate block;
extern crate trie;
#[macro_use]
extern crate lazy_static;
extern crate jsonrpc_core;
extern crate jsonrpc_http_server;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod miner;
mod rpc;

use std::thread;

fn main() {
    thread::spawn(|| {
        miner::mine_loop();
    });

    rpc::rpc_loop(&"127.0.0.1:9545".parse().unwrap());
}
