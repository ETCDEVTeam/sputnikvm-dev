use super::{EthereumRPC, DebugRPC, Either, RPCTransaction, RPCTrace, RPCStep, RPCBlock, RPCLog, RPCReceipt, RPCLogFilter, RPCBlockTrace, RPCDump, RPCDumpAccount, RPCTraceConfig};
use super::util::*;
use super::filter::*;
use super::serialize::*;

use error::Error;
use miner::MinerState;

use rlp::{self, UntrustedRlp};
use bigint::{M256, U256, H256, H2048, Address, Gas};
use hexutil::{read_hex, to_hex};
use block::{Block, TotalHeader, Account, Log, Receipt, FromKey, Transaction, UnsignedTransaction, TransactionAction};
use trie::{Database, DatabaseGuard, FixedSecureTrie};
use blockchain::chain::HeaderHash;
use sputnikvm::{AccountChange, ValidTransaction, SeqTransactionVM, VM, VMStatus, Memory, MachineStatus, HeaderParams, Patch};
use sputnikvm_stateful::MemoryStateful;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::collections::HashMap;
use std::marker::PhantomData;

use jsonrpc_macros::Trailing;

pub struct MinerEthereumRPC<P: Patch + Send> {
    filter: Mutex<FilterManager>,
    state: Arc<Mutex<MinerState>>,
    channel: Sender<bool>,
    _patch: PhantomData<P>,
}

pub struct MinerDebugRPC<P: Patch + Send> {
    state: Arc<Mutex<MinerState>>,
    _patch: PhantomData<P>,
}

unsafe impl<P: Patch + Send> Sync for MinerEthereumRPC<P> { }
unsafe impl<P: Patch + Send> Sync for MinerDebugRPC<P> { }

impl<P: Patch + Send> MinerEthereumRPC<P> {
    pub fn new(state: Arc<Mutex<MinerState>>, channel: Sender<bool>) -> Self {
        MinerEthereumRPC {
            filter: Mutex::new(FilterManager::new(state.clone())),
            channel,
            state,
            _patch: PhantomData,
        }
    }
}

impl<P: Patch + Send> MinerDebugRPC<P> {
    pub fn new(state: Arc<Mutex<MinerState>>) -> Self {
        MinerDebugRPC {
            state,
            _patch: PhantomData,
        }
    }
}

impl<P: 'static + Patch + Send> EthereumRPC for MinerEthereumRPC<P> {
    fn client_version(&self) -> Result<String, Error> {
        Ok("sputnikvm-dev/v0.1".to_string())
    }

    fn sha3(&self, data: Bytes) -> Result<Hex<H256>, Error> {
        use sha3::{Digest, Keccak256};
        Ok(Hex(H256::from(Keccak256::digest(&data.0).as_slice())))
    }

    fn network_id(&self) -> Result<String, Error> {
        Ok(format!("{}", 1))
    }

    fn is_listening(&self) -> Result<bool, Error> {
        Ok(false)
    }

    fn peer_count(&self) -> Result<Hex<usize>, Error> {
        Ok(Hex(0))
    }

    fn protocol_version(&self) -> Result<String, Error> {
        Ok(format!("{}", 63))
    }

    fn is_syncing(&self) -> Result<bool, Error> {
        Ok(false)
    }

    fn coinbase(&self) -> Result<Hex<Address>, Error> {
        Ok(Hex(Address::default()))
    }

    fn is_mining(&self) -> Result<bool, Error> {
        Ok(true)
    }

    fn hashrate(&self) -> Result<String, Error> {
        Ok(format!("{}", 0))
    }

    fn gas_price(&self) -> Result<Hex<Gas>, Error> {
        Ok(Hex(Gas::zero()))
    }

    fn accounts(&self) -> Result<Vec<Hex<Address>>, Error> {
        let state = self.state.lock().unwrap();

        Ok(state.accounts().iter().map(|key| {
            Address::from_secret_key(key).unwrap()
        }).map(|address| {
            Hex(address)
        }).collect())
    }

    fn block_number(&self) -> Result<Hex<usize>, Error> {
        let state = self.state.lock().unwrap();

        Ok(Hex(state.block_height()))
    }

    fn balance(&self, address: Hex<Address>, block: Trailing<String>) -> Result<Hex<U256>, Error> {
        let state = self.state.lock().unwrap();

        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);
        let stateful = state.stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address.0);
        match account {
            Some(account) => {
                Ok(Hex(account.balance))
            },
            None => {
                Ok(Hex(U256::zero()))
            },
        }
    }

    fn storage_at(&self, address: Hex<Address>, index: Hex<U256>, block: Trailing<String>) -> Result<Hex<M256>, Error> {
        let state = self.state.lock().unwrap();

        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);
        let stateful = state.stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address.0);
        match account {
            Some(account) => {
                let storage = stateful.storage_state_of(account.storage_root);
                let value = storage.get(&H256::from(index.0)).unwrap_or(M256::zero());
                Ok(Hex(value))
            },
            None => {
                Ok(Hex(M256::zero()))
            },
        }
    }

    fn transaction_count(&self, address: Hex<Address>, block: Trailing<String>) -> Result<Hex<U256>, Error> {
        let state = self.state.lock().unwrap();

        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);
        let stateful = state.stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address.0);
        match account {
            Some(account) => {
                Ok(Hex(account.nonce))
            },
            None => {
                Ok(Hex(U256::zero()))
            },
        }
    }

    fn block_transaction_count_by_hash(&self, block: Hex<H256>) -> Result<Option<Hex<usize>>, Error> {
        let state = self.state.lock().unwrap();

        let block = match state.get_block_by_hash(block.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(Hex(block.transactions.len())))
    }

    fn block_transaction_count_by_number(&self, number: String) -> Result<Option<Hex<usize>>, Error> {
        let state = self.state.lock().unwrap();

        let number = match from_block_number(&state, number) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = state.get_block_by_number(number);

        Ok(Some(Hex(block.transactions.len())))
    }

    fn block_uncles_count_by_hash(&self, block: Hex<H256>) -> Result<Option<Hex<usize>>, Error> {
        let state = self.state.lock().unwrap();

        let block = match state.get_block_by_hash(block.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(Hex(block.ommers.len())))
    }

    fn block_uncles_count_by_number(&self, number: String) -> Result<Option<Hex<usize>>, Error> {
        let state = self.state.lock().unwrap();

        let number = match from_block_number(&state, number) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = state.get_block_by_number(number);

        Ok(Some(Hex(block.ommers.len())))
    }

    fn code(&self, address: Hex<Address>, block: Trailing<String>) -> Result<Bytes, Error> {
        let state = self.state.lock().unwrap();

        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);
        let stateful = state.stateful();
        let trie = stateful.state_of(block.header.state_root);

        let account: Option<Account> = trie.get(&address.0);
        match account {
            Some(account) => {
                Ok(Bytes(stateful.code(account.code_hash).unwrap()))
            },
            None => {
                Ok(Bytes(Vec::new()))
            },
        }
    }

    fn sign(&self, address: Hex<Address>, message: Bytes) -> Result<Bytes, Error> {
        use sha3::{Digest, Keccak256};
        use secp256k1::{SECP256K1, Message};

        let state = self.state.lock().unwrap();

        let mut signing_message = Vec::new();

        signing_message.extend("Ethereum Signed Message:\n".as_bytes().iter().cloned());
        signing_message.extend(format!("0x{:x}\n", message.0.len()).as_bytes().iter().cloned());
        signing_message.extend(message.0.iter().cloned());

        let hash = H256::from(Keccak256::digest(&signing_message).as_slice());
        let secret_key = {
            let mut secret_key = None;
            for key in state.accounts() {
                if Address::from_secret_key(&key)? == address.0 {
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

        Ok(Bytes(ret))
    }

    fn send_transaction(&self, transaction: RPCTransaction) -> Result<Hex<H256>, Error> {
        let mut state = self.state.lock().unwrap();


        let (valid, transaction) = {
            let stateful = state.stateful();
            let transaction = to_signed_transaction(&state, transaction, &stateful)?;
            let valid = stateful.to_valid::<P>(transaction.clone())?;

            (valid, transaction)
        };

        let hash = state.append_pending_transaction(transaction);
        self.channel.send(true);
        Ok(Hex(hash))
    }

    fn send_raw_transaction(&self, data: Bytes) -> Result<Hex<H256>, Error> {
        let mut state = self.state.lock().unwrap();

        let rlp = UntrustedRlp::new(&data.0);
        let transaction: Transaction = rlp.as_val()?;

        {
            let stateful = state.stateful();
            stateful.to_valid::<P>(transaction.clone())?;
        }

        let hash = state.append_pending_transaction(transaction);
        self.channel.send(true);
        Ok(Hex(hash))
    }

    fn call(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<Bytes, Error> {
        let state = self.state.lock().unwrap();

        let stateful = state.stateful();

        let valid = to_valid_transaction(&state, transaction, &stateful)?;
        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);

        let vm: SeqTransactionVM<P> = stateful.call(
            valid, HeaderParams::from(&block.header),
            &state.get_last_256_block_hashes());

        Ok(Bytes(vm.out().into()))
    }

    fn estimate_gas(&self, transaction: RPCTransaction, block: Trailing<String>) -> Result<Hex<Gas>, Error> {
        let state = self.state.lock().unwrap();

        let stateful = state.stateful();

        let valid = to_valid_transaction(&state, transaction, &stateful)?;
        let block = from_block_number(&state, block)?;

        let block = state.get_block_by_number(block);

        let vm: SeqTransactionVM<P> = stateful.call(
            valid, HeaderParams::from(&block.header),
            &state.get_last_256_block_hashes());

        Ok(Hex(vm.real_used_gas()))
    }

    fn block_by_hash(&self, hash: Hex<H256>, full: bool) -> Result<Option<RPCBlock>, Error> {
        let state = self.state.lock().unwrap();

        let block = match state.get_block_by_hash(hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let total = match state.get_total_header_by_hash(hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(to_rpc_block(block, total, full)))
    }

    fn block_by_number(&self, number: String, full: bool) -> Result<Option<RPCBlock>, Error> {
        let state = self.state.lock().unwrap();

        let number = match from_block_number(&state, Some(number)) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = state.get_block_by_number(number);
        let total = match state.get_total_header_by_hash(block.header.header_hash()) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(to_rpc_block(block, total, full)))
    }

    fn transaction_by_hash(&self, hash: Hex<H256>) -> Result<Option<RPCTransaction>, Error> {
        let state = self.state.lock().unwrap();

        let transaction = match state.get_transaction_by_hash(hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = match state.get_transaction_block_hash_by_hash(hash.0) {
            Ok(block_hash) => state.get_block_by_hash(block_hash).ok(),
            Err(_) => None,
        };

        Ok(Some(to_rpc_transaction(transaction, block.as_ref())))
    }

    fn transaction_by_block_hash_and_index(&self, block_hash: Hex<H256>, index: Hex<U256>) -> Result<Option<RPCTransaction>, Error> {
        let state = self.state.lock().unwrap();

        let block = match state.get_block_by_hash(block_hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if index.0.as_usize() >= block.transactions.len() {
            return Ok(None);
        }
        let transaction = block.transactions[index.0.as_usize()].clone();

        Ok(Some(to_rpc_transaction(transaction, Some(&block))))
    }

    fn transaction_by_block_number_and_index(&self, number: String, index: Hex<U256>) -> Result<Option<RPCTransaction>, Error> {
        let state = self.state.lock().unwrap();

        let number = match from_block_number(&state, Some(number)) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = state.get_block_by_number(number);
        if index.0.as_usize() >= block.transactions.len() {
            return Ok(None);
        }
        let transaction = block.transactions[index.0.as_usize()].clone();

        Ok(Some(to_rpc_transaction(transaction, Some(&block))))
    }

    fn transaction_receipt(&self, hash: Hex<H256>) -> Result<Option<RPCReceipt>, Error> {
        let state = self.state.lock().unwrap();

        let receipt = match state.get_receipt_by_transaction_hash(hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let transaction = match state.get_transaction_by_hash(hash.0) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let block = match state.get_transaction_block_hash_by_hash(hash.0) {
            Ok(val) => state.get_block_by_hash(val).ok(),
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        if block.is_none() {
            Ok(None)
        } else {
            Ok(Some(to_rpc_receipt(&state, receipt, &transaction, &block.unwrap())?))
        }
    }

    fn uncle_by_block_hash_and_index(&self, block_hash: Hex<H256>, index: Hex<U256>) -> Result<Option<RPCBlock>, Error> {
        let state = self.state.lock().unwrap();

        let index = index.0.as_usize();
        let block_hash = block_hash.0;
        let block = match state.get_block_by_hash(block_hash) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let uncle_hash = block.ommers[index].header_hash();
        let uncle = match state.get_block_by_hash(uncle_hash) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let total = match state.get_total_header_by_hash(uncle_hash) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(to_rpc_block(uncle, total, false)))
    }

    fn uncle_by_block_number_and_index(&self, block_number: String, index: Hex<U256>) -> Result<Option<RPCBlock>, Error> {
        let state = self.state.lock().unwrap();

        let block_number = match from_block_number(&state, Some(block_number)) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let index = index.0.as_usize();
        let block = state.get_block_by_number(block_number);
        let uncle_hash = block.ommers[index].header_hash();
        let uncle = match state.get_block_by_hash(uncle_hash) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let total = match state.get_total_header_by_hash(uncle_hash) {
            Ok(val) => val,
            Err(Error::NotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(to_rpc_block(uncle, total, false)))
    }

    fn compilers(&self) -> Result<Vec<String>, Error> {
        Ok(Vec::new())
    }

    fn new_filter(&self, log: RPCLogFilter) -> Result<String, Error> {
        let filter = {
            let state = self.state.lock().unwrap();
            from_log_filter(&state, log)?
        };
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
        let state = self.state.lock().unwrap();

        match from_log_filter(&state, log) {
            Ok(filter) => Ok(get_logs(&state, filter)?),
            Err(_) => Ok(Vec::new()),
        }
    }
}

impl<P: 'static + Patch + Send> DebugRPC for MinerDebugRPC<P> {
    fn block_rlp(&self, number: usize) -> Result<Bytes, Error> {
        let state = self.state.lock().unwrap();

        if number > state.block_height() {
            return Err(Error::NotFound);
        }

        let block = state.get_block_by_number(number);
        Ok(Bytes(rlp::encode(&block).to_vec()))
    }

    fn trace_transaction(&self, hash: Hex<H256>, config: Trailing<RPCTraceConfig>) -> Result<RPCTrace, Error> {
        let config = config.unwrap_or(RPCTraceConfig::default());
        let state = self.state.lock().unwrap();

        let transaction = state.get_transaction_by_hash(hash.0)?;
        let block = state.get_block_by_hash(state.get_transaction_block_hash_by_hash(hash.0)?)?;
        let last_block = state.get_block_by_number(if block.header.number == U256::zero() { 0 } else { block.header.number.as_usize() - 1 });
        let last_hashes = state.get_last_256_block_hashes_by_number(block.header.number.as_usize());

        let mut stateful: MemoryStateful<'static> = state.stateful_at(last_block.header.state_root);
        for other_transaction in &block.transactions {
            if other_transaction != &transaction {
                let valid = stateful.to_valid::<P>(transaction.clone())?;
                let _: SeqTransactionVM<P> =
                    stateful.execute::<_, P>(valid, HeaderParams::from(&block.header), &last_hashes);
            } else {
                break;
            }
        }

        let (steps, vm) = replay_transaction::<P>(&stateful, transaction, &block, &last_hashes, &config)?;

        let gas = Hex(vm.real_used_gas());
        let return_value = Bytes(vm.out().into());

        Ok(RPCTrace {
            gas, return_value,
            struct_logs: steps,
        })
    }

    fn trace_block(&self, block_rlp: Bytes, config: Trailing<RPCTraceConfig>) -> Result<RPCBlockTrace, Error> {
        let config = config.unwrap_or(RPCTraceConfig::default());
        let state = self.state.lock().unwrap();
        let block: Block = UntrustedRlp::new(&block_rlp.0).as_val()?;
        let last_block = state.get_block_by_number(if block.header.number == U256::zero() { 0 } else { block.header.number.as_usize() - 1 });
        let last_hashes = state.get_last_256_block_hashes_by_number(block.header.number.as_usize());

        let mut stateful: MemoryStateful<'static> = state.stateful_at(last_block.header.state_root);
        let mut steps = Vec::new();
        for transaction in block.transactions.clone() {
            let (mut local_steps, vm) = replay_transaction::<P>(&stateful, transaction,
                                                                &block, &last_hashes,
                                                                &config)?;
            steps.append(&mut local_steps);
            let mut accounts = Vec::new();
            for account in vm.accounts() {
                accounts.push(account.clone());
            }
            stateful.transit(&accounts);
        }

        Ok(RPCBlockTrace {
            struct_logs: steps
        })
    }

    fn trace_block_by_number(&self, number: usize, config: Trailing<RPCTraceConfig>) -> Result<RPCBlockTrace, Error> {
        let config = config.unwrap_or(RPCTraceConfig::default());
        let state = self.state.lock().unwrap();
        if number > state.block_height() {
            return Err(Error::NotFound);
        }
        let block: Block = state.get_block_by_number(number);
        let last_block = state.get_block_by_number(if block.header.number == U256::zero() { 0 } else { block.header.number.as_usize() - 1 });
        let last_hashes = state.get_last_256_block_hashes_by_number(block.header.number.as_usize());

        let mut stateful: MemoryStateful<'static> = state.stateful_at(last_block.header.state_root);
        let mut steps = Vec::new();
        for transaction in block.transactions.clone() {
            let (mut local_steps, vm) = replay_transaction::<P>(&stateful, transaction,
                                                                &block, &last_hashes,
                                                                &config)?;
            steps.append(&mut local_steps);
            let mut accounts = Vec::new();
            for account in vm.accounts() {
                accounts.push(account.clone());
            }
            stateful.transit(&accounts);
        }

        Ok(RPCBlockTrace {
            struct_logs: steps
        })
    }

    fn trace_block_by_hash(&self, hash: Hex<H256>, config: Trailing<RPCTraceConfig>) -> Result<RPCBlockTrace, Error> {
        let config = config.unwrap_or(RPCTraceConfig::default());
        let state = self.state.lock().unwrap();
        let block: Block = state.get_block_by_hash(hash.0)?;
        let last_block = state.get_block_by_number(if block.header.number == U256::zero() { 0 } else { block.header.number.as_usize() - 1 });
        let last_hashes = state.get_last_256_block_hashes_by_number(block.header.number.as_usize());

        let mut stateful: MemoryStateful<'static> = state.stateful_at(last_block.header.state_root);
        let mut steps = Vec::new();
        for transaction in block.transactions.clone() {
            let (mut local_steps, vm) = replay_transaction::<P>(&stateful, transaction,
                                                                &block, &last_hashes,
                                                                &config)?;
            steps.append(&mut local_steps);
            let mut accounts = Vec::new();
            for account in vm.accounts() {
                accounts.push(account.clone());
            }
            stateful.transit(&accounts);
        }

        Ok(RPCBlockTrace {
            struct_logs: steps
        })
    }

    fn trace_block_from_file(&self, path: String, config: Trailing<RPCTraceConfig>) -> Result<RPCBlockTrace, Error> {
        use std::fs::File;
        use std::io::Read;

        let config = config.unwrap_or(RPCTraceConfig::default());
        let mut file = File::open(path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();

        let state = self.state.lock().unwrap();
        let block: Block = UntrustedRlp::new(&buffer).as_val()?;
        let last_block = state.get_block_by_number(if block.header.number == U256::zero() { 0 } else { block.header.number.as_usize() - 1 });
        let last_hashes = state.get_last_256_block_hashes_by_number(block.header.number.as_usize());

        let mut stateful: MemoryStateful<'static> = state.stateful_at(last_block.header.state_root);
        let mut steps = Vec::new();
        for transaction in block.transactions.clone() {
            let (mut local_steps, vm) = replay_transaction::<P>(&stateful, transaction,
                                                                &block, &last_hashes,
                                                                &config)?;
            steps.append(&mut local_steps);
            let mut accounts = Vec::new();
            for account in vm.accounts() {
                accounts.push(account.clone());
            }
            stateful.transit(&accounts);
        }

        Ok(RPCBlockTrace {
            struct_logs: steps
        })
    }

    fn dump_block(&self, number: usize) -> Result<RPCDump, Error> {
        let state = self.state.lock().unwrap();
        let block: Block = state.get_block_by_number(number);

        let mut accounts = HashMap::new();
        let database = state.stateful().database();
        let trie: FixedSecureTrie<_, Address, Account> = database.create_fixed_secure_trie(block.header.state_root);
        let code_hashes = database.create_guard();

        for (address, storage) in state.dump_accounts(number) {
            let mut rpc_storage = HashMap::new();
            for (key, value) in storage {
                rpc_storage.insert(Hex(key), Hex(value));
            }

            let account = trie.get(&address).unwrap();
            let code = code_hashes.get(account.code_hash).unwrap();

            accounts.insert(Hex(address), RPCDumpAccount {
                balance: Hex(account.balance),
                code: Bytes(code),
                code_hash: Hex(account.code_hash),
                nonce: Hex(account.nonce),
                root: Hex(account.storage_root),
                storage: rpc_storage,
            });
        }

        Ok(RPCDump {
            accounts,
            root: Hex(block.header.state_root)
        })
    }
}
