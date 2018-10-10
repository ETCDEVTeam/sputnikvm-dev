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

extern crate sputnikvm_network_classic;
extern crate sputnikvm_network_foundation;
extern crate sputnikvm_network_ubiq;
extern crate sputnikvm_network_ellaism;
extern crate sputnikvm_network_expanse;
extern crate sputnikvm_network_musicoin;

#[cfg(feature = "frontend")]
extern crate hyper;

mod error;
mod miner;
mod rpc;

#[cfg(feature = "frontend")]
mod assets;

use miner::{MinerState, MineMode};
use rand::os::OsRng;
use secp256k1::key::{PublicKey, SecretKey};
use secp256k1::SECP256K1;
use bigint::U256;
use hexutil::*;
use std::thread;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use sputnikvm::Patch;

use sputnikvm_network_classic::{
    MainnetEIP160Patch as PClassicEIP160,
    MainnetEIP150Patch as PClassicEIP150,
    MainnetFrontierPatch as PClassicFrontier,
    MainnetHomesteadPatch as PClassicHomestead,
    MordenEIP160Patch as PMordenEIP160,
    MordenEIP150Patch as PMordenEIP150,
    MordenFrontierPatch as PMordenFrontier,
    MordenHomesteadPatch as PMordenHomestead,
};
use sputnikvm_network_foundation::{
    FrontierPatch as PFoundationFrontier,
    HomesteadPatch as PFoundationHomestead,
    EIP150Patch as PFoundationEIP150,
    SpuriousDragonPatch as PFoundationSpuriousDragon,
    ByzantiumPatch as PFoundationByzantium,
};
use sputnikvm_network_ellaism::{
    MainnetEIP160Patch as PEllaismEIP160,
};
use sputnikvm_network_expanse::{
    FrontierPatch as PExpanseFrontier,
    HomesteadPatch as PExpanseHomestead,
    SpuriousDragonPatch as PExpanseSpuriousDragon,
    ByzantiumPatch as PExpanseByzantium,
};
use sputnikvm_network_musicoin::{
    MainnetFrontierPatch as PMusicoinFrontier,
    MainnetHomesteadPatch as PMusicoinHomestead,
};
use sputnikvm_network_ubiq::{
    SpuriousDragonPatch as PUbiqSpuriousDragon,
};

fn main() {
    env_logger::init();

    let matches = clap_app!(
        svmdev =>
            (version: env!("CARGO_PKG_VERSION"))
            (author: "Ethereum Classic Contributors")
            (about: "SputnikVM Development Environment, a replacement for ethereumjs-testrpc.")
            (@arg PRIVATE_KEY: -k --private +takes_value "Private key for the account to be generated, if not provided, a random private key will be generated.")
            (@arg BALANCE: -b --balance +takes_value "Balance in Wei for the account to be generated, default is 0x10000000000000000000000000000.")
            (@arg LISTEN: -l --listen +takes_value "Listen address and port for the RPC, e.g. 127.0.0.1:8545.")
            (@arg ACCOUNTS: -a --accounts +takes_value "Additional accounts to be generated, default to 9.")
            (@arg CHAIN: -c --chain +takes_value "Specify the chain to use. Refer to the documentation for a full list of valid values.")
            (@arg MINE_MODE: -m --minemode +takes_value "Specify the mining mode by number of transactions per block: [AllPending, OnePerBlock]")
    ).get_matches();

    match matches.value_of("CHAIN") {
        None => with_patch::<PClassicEIP160>(matches),

        Some("classic") => with_patch::<PClassicEIP160>(matches),
        Some("classic-eip160") => with_patch::<PClassicEIP160>(matches),
        Some("classic-eip150") => with_patch::<PClassicEIP150>(matches),
        Some("classic-homestead") => with_patch::<PClassicHomestead>(matches),
        Some("classic-frontier") => with_patch::<PClassicFrontier>(matches),

        Some("morden") => with_patch::<PMordenEIP160>(matches),
        Some("morden-eip160") => with_patch::<PMordenEIP160>(matches),
        Some("morden-eip150") => with_patch::<PMordenEIP150>(matches),
        Some("morden-homestead") => with_patch::<PMordenHomestead>(matches),
        Some("morden-frontier") => with_patch::<PMordenFrontier>(matches),

        Some("foundation") => with_patch::<PFoundationByzantium>(matches),
        Some("foundation-byzantium") => with_patch::<PFoundationByzantium>(matches),
        Some("foundation-spurious-dragon") => with_patch::<PFoundationSpuriousDragon>(matches),
        Some("foundation-eip150") => with_patch::<PFoundationEIP150>(matches),
        Some("foundation-homestead") => with_patch::<PFoundationHomestead>(matches),
        Some("foundation-frontier") => with_patch::<PFoundationFrontier>(matches),

        Some("ellaism") => with_patch::<PEllaismEIP160>(matches),
        Some("ellaism-eip160") => with_patch::<PEllaismEIP160>(matches),

        Some("expanse") => with_patch::<PExpanseByzantium>(matches),
        Some("expanse-byzantium") => with_patch::<PExpanseByzantium>(matches),
        Some("expanse-spurious-dragon") => with_patch::<PExpanseSpuriousDragon>(matches),
        Some("expanse-homestead") => with_patch::<PExpanseHomestead>(matches),
        Some("expanse-frontier") => with_patch::<PExpanseFrontier>(matches),

        Some("musicoin") => with_patch::<PMusicoinHomestead>(matches),
        Some("musicoin-homestead") => with_patch::<PMusicoinHomestead>(matches),
        Some("musicoin-frontier") => with_patch::<PMusicoinFrontier>(matches),

        Some("ubiq") => with_patch::<PUbiqSpuriousDragon>(matches),
        Some("ubiq-spurious-dragon") => with_patch::<PUbiqSpuriousDragon>(matches),

        _ => panic!("Unsupported chain."),
    }
}

fn with_patch<'a, P: 'static + Patch + Send>(matches: clap::ArgMatches<'a>) {
    let mut rng = OsRng::new().unwrap();

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

    let mine_mode = match matches.value_of("MINE_MODE") {
        Some(mode) => match mode.to_lowercase().as_ref() {
            "allpending" => MineMode::AllPending,
            "oneperblock" => MineMode::OnePerBlock,
            other => panic!("MINE_MODE should be either AllPending or OnePerBlock, got {}", other),
        },
        None => MineMode::AllPending
    };

    let mut genesis = Vec::new();
    genesis.push((secret_key, balance));

    for _ in 0..accounts_len {
        genesis.push((SecretKey::new(&SECP256K1, &mut rng), balance));
    }

    let (sender, receiver) = channel::<bool>();

    let state = miner::make_state::<P>(genesis);

    let miner_arc = Arc::new(Mutex::new(state));
    let rpc_arc = miner_arc.clone();

    thread::spawn(move || {
        miner::mine_loop::<P>(miner_arc, receiver, mine_mode);
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

    rpc::rpc_loop::<P>(
        rpc_arc,
        &matches.value_of("LISTEN").unwrap_or("127.0.0.1:8545").parse().unwrap(),
        sender);
}
