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

mod miner;

use std::thread;

fn main() {
    thread::spawn(|| {
        miner::mine_loop();
    });

    loop { }
}
