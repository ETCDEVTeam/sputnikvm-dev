use super::Error;
use miner;

use bigint::Address;
use hexutil::{read_hex, to_hex};
use block::FromKey;

pub fn web3_client_version(_params: ()) -> Result<&'static str, Error> {
    Ok("sputnikvm-dev/v0.1")
}

pub fn web3_sha3((data,): (String,)) -> Result<String, Error> {
    use sha3::{Digest, Keccak256};
    Ok(to_hex(Keccak256::digest(&read_hex(&data)?).as_slice()))
}

//

pub fn net_version(_: ()) -> Result<String, Error> {
    Ok(format!("{}", 1))
}

pub fn net_listening(_: ()) -> Result<bool, Error> {
    Ok(false)
}

pub fn net_peer_count(_: ()) -> Result<String, Error> {
    Ok(format!("0x{:x}", 0))
}

pub fn eth_protocol_version(_: ()) -> Result<String, Error> {
    Ok(format!("{}", 63))
}

pub fn eth_syncing(_: ()) -> Result<bool, Error> {
    Ok(false)
}

pub fn eth_coinbase(_: ()) -> Result<String, Error> {
    Ok(format!("0x{:x}", Address::default()))
}

pub fn eth_mining(_: ()) -> Result<bool, Error> {
    Ok(true)
}

pub fn eth_hashrate(_: ()) -> Result<String, Error> {
    Ok(format!("{}", 0))
}

pub fn eth_gas_price(_: ()) -> Result<String, Error> {
    Ok(format!("0x{:x}", 0))
}

pub fn eth_accounts(_: ()) -> Result<Vec<String>, Error> {
    Ok(miner::accounts().iter().map(|key| {
        Address::from_secret_key(key).unwrap()
    }).map(|address| {
        format!("0x{:x}", address)
    }).collect())
}
