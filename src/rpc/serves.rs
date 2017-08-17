use super::Error;
use hexutil::{read_hex, to_hex};

pub fn web3_client_version(_params: ()) -> Result<&'static str, Error> {
    Ok("sputnikvm-dev/v0.1")
}

pub fn web3_sha3((data,): (String,)) -> Result<String, Error> {
    use sha3::{Digest, Keccak256};
    Ok(to_hex(Keccak256::digest(&read_hex(&data)?).as_slice()))
}
