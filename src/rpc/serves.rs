use super::{EthereumRPC, Either, RPCTransaction, RPCBlock, Error};
use miner;

use rlp::{self, UntrustedRlp};
use bigint::{M256, U256, H256, H2048, Address, Gas};
use hexutil::{read_hex, to_hex};
use block::{Block, TotalHeader, Account, FromKey, Transaction, UnsignedTransaction, TransactionAction};
use blockchain::chain::HeaderHash;
use sputnikvm::vm::{self, ValidTransaction, VM};
use std::str::FromStr;

use jsonrpc_macros::Trailing;

fn from_block_number<T: Into<Option<String>>>(value: T) -> Result<usize, Error> {
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

fn to_rpc_transaction(transaction: Transaction, block: Option<&Block>) -> RPCTransaction {
    use sha3::{Keccak256, Digest};
    let hash = H256::from(Keccak256::digest(&rlp::encode(&transaction).to_vec()).as_slice());

    RPCTransaction {
        from: format!("0x{:x}", transaction.caller().unwrap()),
        to: match transaction.action {
            TransactionAction::Call(address) =>
                Some(format!("0x{:x}", address)),
            TransactionAction::Create => None,
        },
        gas: Some(format!("0x{:x}", transaction.gas_limit)),
        gas_price: Some(format!("0x{:x}", transaction.gas_price)),
        value: Some(format!("0x{:x}", transaction.value)),
        data: to_hex(&transaction.input),
        nonce: Some(format!("0x{:x}", transaction.nonce)),

        hash: Some(format!("0x{:x}", hash)),
        block_hash: block.map(|b| format!("0x{:x}", b.header.header_hash())),
        block_number: block.map(|b| format!("0x{:x}", b.header.number)),
        transaction_index: {
            if block.is_some() {
                let block = block.unwrap();
                let mut i = 0;
                let mut found = false;
                for transaction in &block.transactions {
                    let other_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
                    if hash == other_hash {
                        found = true;
                        break;
                    }
                    i += 1;
                }
                if found {
                    Some(format!("0x{:x}", i))
                } else {
                    None
                }
            } else {
                None
            }
        },
    }
}

fn to_rpc_block(block: Block, total_header: TotalHeader, full_transactions: bool) -> RPCBlock {
    use sha3::{Keccak256, Digest};
    let logs_bloom: H2048 = block.header.logs_bloom.clone().into();

    RPCBlock {
        number: format!("0x{:x}", block.header.number),
        hash: format!("0x{:x}", block.header.header_hash()),
        parent_hash: format!("0x{:x}", block.header.parent_hash()),
        nonce: format!("0x{:x}", block.header.nonce),
        sha3_uncles: format!("0x{:x}", block.header.ommers_hash),
        logs_bloom: format!("0x{:x}", logs_bloom),
        transactions_root: format!("0x{:x}", block.header.transactions_root),
        state_root: format!("0x{:x}", block.header.state_root),
        receipts_root: format!("0x{:x}", block.header.receipts_root),
        miner: format!("0x{:x}", block.header.beneficiary),
        difficulty: format!("0x{:x}", block.header.difficulty),
        total_difficulty: format!("0x{:x}", total_header.total_difficulty()),

        // TODO: change this to the correct one after the Typhoon is over...
        extra_data: to_hex(&rlp::encode(&block.header.extra_data).to_vec()),

        size: format!("0x{:x}", rlp::encode(&block.header).to_vec().len()),
        gas_limit: format!("0x{:x}", block.header.gas_limit),
        gas_used: format!("0x{:x}", block.header.gas_used),
        timestamp: format!("0x{:x}", block.header.timestamp),
        transactions: if full_transactions {
            Either::Right(block.transactions.iter().map(|t| to_rpc_transaction(t.clone(), Some(&block))).collect())
        } else {
            Either::Left(block.transactions.iter().map(|t| {
                let encoded = rlp::encode(t).to_vec();
                format!("0x{:x}", H256::from(Keccak256::digest(&encoded).as_slice()))
            }).collect())
        },
        uncles: block.ommers.iter().map(|u| format!("0x{:x}", u.header_hash())).collect(),
    }
}

fn to_signed_transaction(transaction: RPCTransaction) -> Result<Transaction, Error> {
    let address = Address::from_str(&transaction.from)?;
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
    let block = miner::get_block_by_number(miner::block_height());
    let database = miner::trie_database();
    let trie = database.create_trie(block.header.state_root);

    let account: Option<Account> = trie.get(&address);

    let unsigned = UnsignedTransaction {
        nonce: match transaction.nonce {
            Some(val) => U256::from_str(&val)?,
            None => {
                account.as_ref().map(|account| account.nonce).unwrap_or(U256::zero())
            }
        },
        gas_price: match transaction.gas_price {
            Some(val) => Gas::from_str(&val)?,
            None => Gas::zero(),
        },
        gas_limit: match transaction.gas {
            Some(val) => Gas::from_str(&val)?,
            None => Gas::from(90000u64),
        },
        action: match transaction.to {
            Some(val) => TransactionAction::Call(Address::from_str(&val)?),
            None => TransactionAction::Create,
        },
        value: match transaction.value {
            Some(val) => U256::from_str(&val)?,
            None => U256::zero(),
        },
        input: read_hex(&transaction.data)?,
        network_id: None,
    };
    let transaction = unsigned.sign(&secret_key);

    Ok(transaction)
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

    fn send_transaction(&self, transaction: RPCTransaction) -> Result<String, Error> {
        let transaction = to_signed_transaction(transaction)?;

        let hash = miner::append_pending_transaction(transaction);
        Ok(format!("0x{:x}", hash))
    }

    fn send_raw_transaction(&self, data: String) -> Result<String, Error> {
        let data = read_hex(&data)?;
        let rlp = UntrustedRlp::new(&data);
        let transaction: Transaction = rlp.as_val()?;

        let hash = miner::append_pending_transaction(transaction);
        Ok(format!("0x{:x}", hash))
    }

    fn call(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<String, Error> {
        let transaction = to_signed_transaction(transaction)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let database = miner::trie_database();
        let trie = database.create_trie(block.header.state_root);

        let valid = miner::to_valid(&database, transaction, &vm::EIP160_PATCH, &trie);
        let vm = miner::call(&database, &block, valid, &vm::EIP160_PATCH, &trie);

        Ok(to_hex(vm.out()))
    }

    fn estimate_gas(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<String, Error> {
        let transaction = to_signed_transaction(transaction)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let database = miner::trie_database();
        let trie = database.create_trie(block.header.state_root);

        let valid = miner::to_valid(&database, transaction, &vm::EIP160_PATCH, &trie);
        let vm = miner::call(&database, &block, valid, &vm::EIP160_PATCH, &trie);

        Ok(format!("0x{:x}", vm.real_used_gas()))
    }

    fn block_by_hash(&self, hash: String, full: bool) -> Result<RPCBlock, Error> {
        let hash = H256::from_str(&hash)?;
        let block = miner::get_block_by_hash(hash);
        let total = miner::get_total_header_by_hash(hash);

        Ok(to_rpc_block(block, total, full))
    }

    fn block_by_number(&self, number: String, full: bool) -> Result<RPCBlock, Error> {
        let number = from_block_number(Some(number))?;
        let block = miner::get_block_by_number(number);
        let total = miner::get_total_header_by_hash(block.header.header_hash());

        Ok(to_rpc_block(block, total, full))
    }

    fn transaction_by_hash(&self, hash: String) -> Result<RPCTransaction, Error> {
        let hash = H256::from_str(&hash)?;
        let transaction = miner::get_transaction_by_hash(hash);
        let block = miner::get_transaction_block_hash_by_hash(hash).map(|block_hash| {
            miner::get_block_by_hash(block_hash)
        });

        Ok(to_rpc_transaction(transaction, block.as_ref()))
    }

    fn transaction_by_block_hash_and_index(&self, block_hash: String, index: String) -> Result<RPCTransaction, Error> {
        let index = U256::from_str(&index)?.as_usize();
        let block_hash = H256::from_str(&block_hash)?;
        let block = miner::get_block_by_hash(block_hash);
        let transaction = block.transactions[index].clone();

        Ok(to_rpc_transaction(transaction, Some(&block)))
    }

    fn transaction_by_block_number_and_index(&self, number: String, index: String) -> Result<RPCTransaction, Error> {
        let index = U256::from_str(&index)?.as_usize();
        let number = U256::from_str(&number)?.as_usize();
        let block = miner::get_block_by_number(number);
        let transaction = block.transactions[index].clone();

        Ok(to_rpc_transaction(transaction, Some(&block)))
    }
}
