use super::{EthereumRPC, Either, RPCTransaction, RPCBlock, RPCLog, RPCReceipt, RPCLogFilter};
use super::util::*;
use super::filter::*;

use error::Error;
use miner;

use rlp::{self, UntrustedRlp};
use bigint::{M256, U256, H256, H2048, Address, Gas};
use hexutil::{read_hex, to_hex};
use block::{Block, TotalHeader, Account, Log, Receipt, FromKey, Transaction, UnsignedTransaction, TransactionAction};
use blockchain::chain::HeaderHash;
use sputnikvm::vm::{self, ValidTransaction, SeqTransactionVM, VM, HeaderParams};
use std::str::FromStr;
use std::sync::Mutex;

use jsonrpc_macros::Trailing;

pub struct MinerEthereumRPC {
    filter: Mutex<FilterManager>,
}

impl MinerEthereumRPC {
    pub fn new() -> Self {
        MinerEthereumRPC {
            filter: Mutex::new(FilterManager::new()),
        }
    }
}

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
        let stateful = miner::stateful();
        let trie = stateful.state_of(block.header.state_root);

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
        let stateful = miner::stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address);
        match account {
            Some(account) => {
                let storage = stateful.storage_state_of(account.storage_root);
                let value = storage.get(&H256::from(index)).unwrap_or(M256::zero());
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
        let block = miner::get_block_by_hash(hash)?;

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
        let block = miner::get_block_by_hash(hash)?;

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
        let stateful = miner::stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address);
        match account {
            Some(account) => {
                Ok(to_hex(&miner::get_hash_raw(account.code_hash)?))
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
                None => return Err(Error::NotFound),
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
        let stateful = miner::stateful();
        let transaction = to_signed_transaction(transaction)?;

        stateful.to_valid(transaction.clone(), &vm::EIP160_PATCH)?;

        let hash = miner::append_pending_transaction(transaction);
        Ok(format!("0x{:x}", hash))
    }

    fn send_raw_transaction(&self, data: String) -> Result<String, Error> {
        let stateful = miner::stateful();
        let data = read_hex(&data)?;
        let rlp = UntrustedRlp::new(&data);
        let transaction: Transaction = rlp.as_val()?;

        stateful.to_valid(transaction.clone(), &vm::EIP160_PATCH)?;

        let hash = miner::append_pending_transaction(transaction);
        Ok(format!("0x{:x}", hash))
    }

    fn call(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<String, Error> {
        let transaction = to_signed_transaction(transaction)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let stateful = miner::stateful();
        let trie = stateful.state_of(block.header.state_root);

        let valid = stateful.to_valid(transaction, &vm::EIP160_PATCH)?;
        let vm: SeqTransactionVM = stateful.call(
            valid, HeaderParams::from(&block.header), &vm::EIP160_PATCH,
            &miner::get_last_256_block_hashes());

        Ok(to_hex(vm.out()))
    }

    fn estimate_gas(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<String, Error> {
        let transaction = to_signed_transaction(transaction)?;
        let block = from_block_number(block)?;

        let block = miner::get_block_by_number(block);
        let stateful = miner::stateful();
        let trie = stateful.state_of(block.header.state_root);

        let valid = stateful.to_valid(transaction, &vm::EIP160_PATCH)?;
        let vm: SeqTransactionVM = stateful.call(
            valid, HeaderParams::from(&block.header), &vm::EIP160_PATCH,
            &miner::get_last_256_block_hashes());

        Ok(format!("0x{:x}", vm.real_used_gas()))
    }

    fn block_by_hash(&self, hash: String, full: bool) -> Result<RPCBlock, Error> {
        let hash = H256::from_str(&hash)?;
        let block = miner::get_block_by_hash(hash)?;
        let total = miner::get_total_header_by_hash(hash)?;

        Ok(to_rpc_block(block, total, full))
    }

    fn block_by_number(&self, number: String, full: bool) -> Result<RPCBlock, Error> {
        let number = from_block_number(Some(number))?;
        let block = miner::get_block_by_number(number);
        let total = miner::get_total_header_by_hash(block.header.header_hash())?;

        Ok(to_rpc_block(block, total, full))
    }

    fn transaction_by_hash(&self, hash: String) -> Result<RPCTransaction, Error> {
        let hash = H256::from_str(&hash)?;
        let transaction = miner::get_transaction_by_hash(hash)?;
        let block = match miner::get_transaction_block_hash_by_hash(hash) {
            Ok(block_hash) => miner::get_block_by_hash(block_hash).ok(),
            Err(_) => None,
        };

        Ok(to_rpc_transaction(transaction, block.as_ref()))
    }

    fn transaction_by_block_hash_and_index(&self, block_hash: String, index: String) -> Result<RPCTransaction, Error> {
        let index = U256::from_str(&index)?.as_usize();
        let block_hash = H256::from_str(&block_hash)?;
        let block = miner::get_block_by_hash(block_hash)?;
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

    fn transaction_receipt(&self, hash: String) -> Result<RPCReceipt, Error> {
        let hash = H256::from_str(&hash)?;
        let receipt = miner::get_receipt_by_hash(hash)?;
        let transaction = miner::get_transaction_by_hash(hash)?;
        let block = match miner::get_transaction_block_hash_by_hash(hash) {
            Ok(block_hash) => miner::get_block_by_hash(block_hash).ok(),
            Err(_) => None,
        };

        if block.is_none() {
            Err(Error::NotFound)
        } else {
            Ok(to_rpc_receipt(receipt, &transaction, &block.unwrap())?)
        }
    }

    fn uncle_by_block_hash_and_index(&self, block_hash: String, index: String) -> Result<RPCBlock, Error> {
        let index = U256::from_str(&index)?.as_usize();
        let block_hash = H256::from_str(&block_hash)?;
        let block = miner::get_block_by_hash(block_hash)?;
        let uncle_hash = block.ommers[index].header_hash();
        let uncle = miner::get_block_by_hash(uncle_hash)?;
        let total = miner::get_total_header_by_hash(uncle_hash)?;

        Ok(to_rpc_block(uncle, total, false))
    }

    fn uncle_by_block_number_and_index(&self, block_number: String, index: String) -> Result<RPCBlock, Error> {
        let block_number = from_block_number(Some(block_number))?;
        let index = U256::from_str(&index)?.as_usize();
        let block = miner::get_block_by_number(block_number);
        let uncle_hash = block.ommers[index].header_hash();
        let uncle = miner::get_block_by_hash(uncle_hash)?;
        let total = miner::get_total_header_by_hash(uncle_hash)?;

        Ok(to_rpc_block(uncle, total, false))
    }

    fn compilers(&self) -> Result<Vec<String>, Error> {
        Ok(Vec::new())
    }

    fn new_filter(&self, log: RPCLogFilter) -> Result<String, Error> {
        let filter = from_log_filter(log)?;
        let id = self.filter.lock().unwrap().install_log_filter(filter);
        Ok(format!("0x{:x}", id))
    }

    fn new_block_filter(&self) -> Result<String, Error> {
        let id = self.filter.lock().unwrap().install_block_filter();
        Ok(format!("0x{:x}", id))
    }

    fn new_pending_transaction_filter(&self) -> Result<String, Error> {
        let id = self.filter.lock().unwrap().install_pending_transaction_filter();
        Ok(format!("0x{:x}", id))
    }

    fn uninstall_filter(&self, id: String) -> Result<bool, Error> {
        let id = U256::from_str(&id)?.as_usize();
        self.filter.lock().unwrap().uninstall_filter(id);
        Ok(true)
    }

    fn filter_changes(&self, id: String) -> Result<Either<Vec<String>, Vec<RPCLog>>, Error> {
        let id = U256::from_str(&id)?.as_usize();
        Ok(self.filter.lock().unwrap().get_changes(id)?)
    }

    fn filter_logs(&self, id: String) -> Result<Vec<RPCLog>, Error> {
        let id = U256::from_str(&id)?.as_usize();
        Ok(self.filter.lock().unwrap().get_logs(id)?)
    }

    fn logs(&self, log: RPCLogFilter) -> Result<Vec<RPCLog>, Error> {
        let filter = from_log_filter(log)?;
        Ok(get_logs(filter)?)
    }
}
