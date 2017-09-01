use super::{EthereumRPC, Either, RPCTransaction, RPCBlock, RPCLog, RPCReceipt, RPCTopicFilter, RPCLogFilter};
use super::filter::*;
use error::Error;
use miner;

use rlp::{self, UntrustedRlp};
use bigint::{M256, U256, H256, H2048, Address, Gas};
use hexutil::{read_hex, to_hex};
use block::{Block, TotalHeader, Account, Log, Receipt, FromKey, Transaction, UnsignedTransaction, TransactionAction};
use blockchain::chain::HeaderHash;
use sputnikvm::vm::{self, ValidTransaction, VM};
use sputnikvm_stateful::MemoryStateful;
use std::str::FromStr;

use jsonrpc_macros::Trailing;

pub fn from_block_number<T: Into<Option<String>>>(value: T) -> Result<usize, Error> {
    let value: Option<String> = value.into();

    if value == Some("latest".to_string()) || value == Some("pending".to_string()) || value == None {
        Ok(miner::block_height())
    } else if value == Some("earliest".to_string()) {
        Ok(0)
    } else {
        let v: u64 = U256::from(read_hex(&value.unwrap())?.as_slice()).into();
        let v = v as usize;
        if v > miner::block_height() {
            Err(Error::NotFound)
        } else {
            Ok(v)
        }
    }
}

pub fn to_rpc_log(receipt: &Receipt, index: usize, transaction: &Transaction, block: &Block) -> RPCLog {
    use sha3::{Keccak256, Digest};

    let transaction_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
    let transaction_index = {
        let mut i = 0;
        let mut found = false;
        for transaction in &block.transactions {
            let other_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
            if transaction_hash == other_hash {
                found = true;
                break;
            }
            i += 1;
        }
        assert!(found);
        i
    };

    RPCLog {
        removed: false,
        log_index: format!("0x{:x}", index),
        transaction_index: format!("0x{:x}", transaction_index),
        transaction_hash: format!("0x{:x}", transaction_hash),
        block_hash: format!("0x{:x}", block.header.header_hash()),
        block_number: format!("0x{:x}", block.header.number),
        data: to_hex(&receipt.logs[index].data),
        topics: receipt.logs[index].topics.iter().map(|t| format!("0x{:x}", t)).collect(),
    }
}

pub fn to_rpc_receipt(receipt: Receipt, transaction: &Transaction, block: &Block) -> Result<RPCReceipt, Error> {
    use sha3::{Keccak256, Digest};

    let transaction_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
    let transaction_index = {
        let mut i = 0;
        let mut found = false;
        for transaction in &block.transactions {
            let other_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
            if transaction_hash == other_hash {
                found = true;
                break;
            }
            i += 1;
        }
        assert!(found);
        i
    };

    let cumulative_gas_used = {
        let mut sum = Gas::zero();

        for i in 0..(transaction_index + 1) {
            let other_hash = H256::from(Keccak256::digest(&rlp::encode(&block.transactions[i]).to_vec()).as_slice());
            sum = sum + miner::get_receipt_by_transaction_hash(other_hash)?.used_gas;
        }
        sum
    };

    let contract_address = {
        if transaction.action == TransactionAction::Create {
            Some(transaction.address().unwrap())
        } else {
            None
        }
    };

    Ok(RPCReceipt {
        transaction_hash: format!("0x{:x}", transaction_hash),
        transaction_index: format!("0x{:x}", transaction_index),
        block_hash: format!("0x{:x}", block.header.header_hash()),
        block_number: format!("0x{:x}", block.header.number),
        cumulative_gas_used: format!("0x{:x}", cumulative_gas_used),
        gas_used: format!("0x{:x}", receipt.used_gas),
        contract_address: contract_address.map(|v| format!("0x{:x}", v)),
        logs: {
            let mut ret = Vec::new();
            for i in 0..receipt.logs.len() {
                ret.push(to_rpc_log(&receipt, i, transaction, block));
            }
            ret
        },
    })
}

pub fn to_rpc_transaction(transaction: Transaction, block: Option<&Block>) -> RPCTransaction {
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

pub fn to_rpc_block(block: Block, total_header: TotalHeader, full_transactions: bool) -> RPCBlock {
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

pub fn to_signed_transaction(transaction: RPCTransaction, stateful: &MemoryStateful) -> Result<Transaction, Error> {
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
            None => return Err(Error::NotFound),
        }
    };
    let block = miner::get_block_by_number(miner::block_height());
    let trie = stateful.state_of(block.header.state_root);

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

pub fn to_valid_transaction(transaction: RPCTransaction, stateful: &MemoryStateful) -> Result<ValidTransaction, Error> {
    let address = Address::from_str(&transaction.from)?;

    let block = miner::get_block_by_number(miner::block_height());
    let trie = stateful.state_of(block.header.state_root);

    let account: Option<Account> = trie.get(&address);

    let valid = ValidTransaction {
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
        caller: Some(address),
    };

    Ok(valid)
}

pub fn from_topic_filter(filter: Option<RPCTopicFilter>) -> Result<TopicFilter, Error> {
    Ok(match filter {
        None => TopicFilter::All,
        Some(RPCTopicFilter::Single(s)) => TopicFilter::Or(vec![
            H256::from_str(&s)?
        ]),
        Some(RPCTopicFilter::Or(ss)) => {
            let mut ret = Vec::new();
            for s in ss {
                ret.push(H256::from_str(&s)?)
            }
            TopicFilter::Or(ret)
        },
    })
}

pub fn from_log_filter(filter: RPCLogFilter) -> Result<LogFilter, Error> {
    Ok(LogFilter {
        from_block: from_block_number(filter.from_block)?,
        to_block: from_block_number(filter.to_block)?,
        address: match filter.address {
            Some(val) => Some(Address::from_str(&val)?),
            None => None,
        },
        topics: match filter.topics {
            Some(topics) => {
                let mut ret = Vec::new();
                for i in 0..4 {
                    if topics.len() > i {
                        ret.push(from_topic_filter(topics[i].clone())?);
                    } else {
                        ret.push(TopicFilter::All);
                    }
                }
                ret
            },
            None => vec![TopicFilter::All, TopicFilter::All, TopicFilter::All, TopicFilter::All],
        },
    })
}
