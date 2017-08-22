use super::{EthereumRPC, Error};
use miner;

use rlp;
use bigint::{M256, U256, H256, Address};
use hexutil::{read_hex, to_hex};
use block::{Account, FromKey};
use std::str::FromStr;

use jsonrpc_macros::Trailing;

fn from_block_number(value: Trailing<String>) -> Result<usize, Error> {
    let value: Option<String> = value.into();

    if value == Some("latest".to_string()) || value == Some("pending".to_string()) || value == None {
        Ok(miner::block_height())
    } else if value == Some("earliest".to_string()) {
        Ok(0)
    } else {
        let v: u64 = U256::from(read_hex(&value.unwrap())?.as_slice()).into();
        Ok(v as usize)
    }
}

pub struct MinerEthereumRPC;

impl EthereumRPC for MinerEthereumRPC {
    fn client_version(&self) -> Result<String, Error> {
        Ok("sputnikvm-dev/v0.1".to_string())
    }

    fn sha3(&self, data: String) -> Result<String, Error> {
        use sha3::{Digest, Keccak256};
        Ok(to_hex(Keccak256::digest(&read_hex(&data)?).as_slice()))
    }

    fn network_id(&self) -> Result<String, Error> {
        Ok(format!("{}", 1))
    }

    fn is_listening(&self) -> Result<bool, Error> {
        Ok(false)
    }

    fn peer_count(&self) -> Result<String, Error> {
        Ok(format!("0x{:x}", 0))
    }

    fn protocol_version(&self) -> Result<String, Error> {
        Ok(format!("{}", 63))
    }

    fn is_syncing(&self) -> Result<bool, Error> {
        Ok(false)
    }

    fn coinbase(&self) -> Result<String, Error> {
        Ok(format!("0x{:x}", Address::default()))
    }

    fn is_mining(&self) -> Result<bool, Error> {
        Ok(true)
    }

    fn hashrate(&self) -> Result<String, Error> {
        Ok(format!("{}", 0))
    }

    fn gas_price(&self) -> Result<String, Error> {
        Ok(format!("0x{:x}", 0))
    }

    fn accounts(&self) -> Result<Vec<String>, Error> {
        Ok(miner::accounts().iter().map(|key| {
            Address::from_secret_key(key).unwrap()
        }).map(|address| {
            format!("0x{:x}", address)
        }).collect())
    }

    fn block_number(&self) -> Result<String, Error> {
        Ok(format!("0x{:x}", miner::block_height()))
    }

    fn balance(&self, address: String, block: Trailing<String>) -> Result<String, Error> {
        let address = Address::from_str(&address)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let database = miner::trie_database();
        let trie = database.create_trie(block.header.state_root);

        let account: Option<Account> = trie.get(&address);
        match account {
            Some(account) => {
                Ok(format!("0x{:x}", account.balance))
            },
            None => {
                Ok(format!("0x{:x}", 0))
            },
        }
    }

    fn storage_at(&self, address: String, index: String, block: Trailing<String>) -> Result<String, Error> {
        let address = Address::from_str(&address)?;
        let index = U256::from_str(&index)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let database = miner::trie_database();
        let trie = database.create_trie(block.header.state_root);

        let account: Option<Account> = trie.get(&address);
        match account {
            Some(account) => {
                let storage = database.create_trie(account.storage_root);
                let value = storage.get(&index).unwrap_or(M256::zero());
                Ok(format!("0x{:x}", value))
            },
            None => {
                Ok(format!("0x{:x}", 0))
            },
        }
    }

    fn transaction_count(&self, address: String, block: Trailing<String>) -> Result<String, Error> {
        let address = Address::from_str(&address)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let mut count = 0;

        for transactions in block.transactions {
            if transactions.caller()? == address {
                count += 1;
            }
        }

        Ok(format!("0x{:x}", count))
    }

    fn block_transaction_count_by_hash(&self, block: String) -> Result<Option<String>, Error> {
        let hash = H256::from_str(&block)?;
        let block = miner::get_block_by_hash(hash);

        // TODO: handle None case
        Ok(Some(format!("0x{:x}", block.transactions.len())))
    }

    fn block_transaction_count_by_number(&self, number: String) -> Result<Option<String>, Error> {
        let number = U256::from_str(&number)?;
        let block = miner::get_block_by_number(number.as_usize());

        // TODO: handle None case
        Ok(Some(format!("0x{:x}", block.transactions.len())))
    }

    fn block_uncles_count_by_hash(&self, block: String) -> Result<Option<String>, Error> {
        let hash = H256::from_str(&block)?;
        let block = miner::get_block_by_hash(hash);

        // TODO: handle None case
        Ok(Some(format!("0x{:x}", block.ommers.len())))
    }

    fn block_uncles_count_by_number(&self, number: String) -> Result<Option<String>, Error> {
        let number = U256::from_str(&number)?;
        let block = miner::get_block_by_number(number.as_usize());

        // TODO: handle None case
        Ok(Some(format!("0x{:x}", block.ommers.len())))
    }

    fn code(&self, address: String, block: Trailing<String>) -> Result<String, Error> {
        let address = Address::from_str(&address)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let database = miner::trie_database();
        let trie = database.create_trie(block.header.state_root);

        let account: Option<Account> = trie.get(&address);
        match account {
            Some(account) => {
                Ok(to_hex(&miner::get_hash_raw(account.code_hash)))
            },
            None => {
                Ok("".to_string())
            },
        }
    }

    fn sign(&self, address: String, message: String) -> Result<String, Error> {
        use sha3::{Digest, Keccak256};
        use secp256k1::{SECP256K1, Message};

        let address = Address::from_str(&address)?;
        let mut signing_message = Vec::new();

        signing_message.extend("Ethereum Signed Message:\n".as_bytes().iter().cloned());
        signing_message.extend(format!("0x{:x}\n", message.as_bytes().len()).as_bytes().iter().cloned());
        signing_message.extend(message.as_bytes().iter().cloned());

        let hash = H256::from(Keccak256::digest(&signing_message).as_slice());
        let secret_key = {
            let mut secret_key = None;
            for key in miner::accounts() {
                if Address::from_secret_key(&key)? == address {
                    secret_key = Some(key);
                }
            }
            match secret_key {
                Some(val) => val,
                None => return Err(Error::AccountNotFound),
            }
        };
        let sign = SECP256K1.sign_recoverable(&Message::from_slice(&hash).unwrap(), &secret_key)?;
        let (rec, sign) = sign.serialize_compact(&SECP256K1);
        let mut ret = Vec::new();
        ret.push(rec.to_i32() as u8);
        ret.extend(sign.as_ref());

        Ok(to_hex(&ret))
    }
}
