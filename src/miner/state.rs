use rlp;
use block::{Receipt, Block, TotalHeader, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use sha3::{Digest, Keccak256};
use secp256k1::key::SecretKey;
use sputnikvm_stateful::{MemoryStateful};

use std::sync::{Mutex, MutexGuard};
use std::collections::HashMap;

lazy_static! {
    static ref ALL_PENDING_TRANSACTION_HASHES: Mutex<Vec<H256>> = Mutex::new(Vec::new());
    static ref PENDING_TRANSACTION_HASHES: Mutex<Vec<H256>> = Mutex::new(Vec::new());
    static ref CURRENT_BLOCK: Mutex<H256> = Mutex::new(H256::default());
    static ref BLOCK_HASHES: Mutex<Vec<H256>> = Mutex::new(Vec::new());
    static ref TRANSACTION_BLOCK_HASHES: Mutex<HashMap<H256, H256>> = Mutex::new(HashMap::new());
    static ref TOTAL_HEADERS: Mutex<HashMap<H256, TotalHeader>> = Mutex::new(HashMap::new());
    static ref HASH_DATABASE: Mutex<HashMap<H256, Vec<u8>>> = Mutex::new(HashMap::new());
    static ref ACCOUNTS: Mutex<Vec<SecretKey>> = Mutex::new(Vec::new());
    static ref STATEFUL: Mutex<MemoryStateful> = Mutex::new(MemoryStateful::default());
}

pub fn append_pending_transaction(transaction: Transaction) -> H256 {
    let value = rlp::encode(&transaction).to_vec();
    let hash = H256::from(Keccak256::digest(&value).as_slice());
    insert_hash_raw(hash, value);

    PENDING_TRANSACTION_HASHES.lock().unwrap().push(hash);
    ALL_PENDING_TRANSACTION_HASHES.lock().unwrap().push(hash);

    hash
}

pub fn clear_pending_transactions() -> Vec<Transaction> {
    let transaction_hashes = {
        let mut pending_transactions = PENDING_TRANSACTION_HASHES.lock().unwrap();
        let ret_hashes = pending_transactions.clone();
        pending_transactions.clear();
        ret_hashes
    };

    let mut transactions = Vec::new();
    for hash in transaction_hashes {
        transactions.push(rlp::decode(&get_hash_raw(hash)))
    }
    transactions
}

pub fn all_pending_transaction_hashes() -> Vec<H256> {
    ALL_PENDING_TRANSACTION_HASHES.lock().unwrap().clone()
}

pub fn insert_hash_raw(key: H256, value: Vec<u8>) {
    HASH_DATABASE.lock().unwrap().insert(key, value);
}

pub fn get_hash_raw(key: H256) -> Vec<u8> {
    HASH_DATABASE.lock().unwrap().get(&key).unwrap().clone()
}

pub fn append_block(block: Block) -> H256 {
    let value = rlp::encode(&block).to_vec();
    let hash = H256::from(Keccak256::digest(&value).as_slice());
    insert_hash_raw(hash, value);

    for transaction in &block.transactions {
        let transaction_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
        TRANSACTION_BLOCK_HASHES.lock().unwrap().insert(transaction_hash, hash);
    }

    let parent_hash = BLOCK_HASHES.lock().unwrap()[block_height()];
    BLOCK_HASHES.lock().unwrap().push(hash);
    let mut total_headers = TOTAL_HEADERS.lock().unwrap();
    if total_headers.len() == 0 {
        total_headers.insert(hash, TotalHeader::from_genesis(block.header.clone()));
    } else {
        let parent = total_headers.get(&parent_hash).unwrap().clone();
        total_headers.insert(hash, TotalHeader::from_parent(block.header.clone(), &parent));
    }
    *CURRENT_BLOCK.lock().unwrap() = hash;

    hash
}

pub fn block_height() -> usize {
    BLOCK_HASHES.lock().unwrap().len() - 1
}

pub fn get_transaction_block_hash_by_hash(key: H256) -> Option<H256> {
    TRANSACTION_BLOCK_HASHES.lock().unwrap().get(&key).map(|v| v.clone())
}

pub fn get_block_by_hash(key: H256) -> Block {
    rlp::decode(&get_hash_raw(key))
}

pub fn get_transaction_by_hash(key: H256) -> Transaction {
    rlp::decode(&get_hash_raw(key))
}

pub fn get_receipt_by_hash(key: H256) -> Receipt {
    rlp::decode(&get_hash_raw(key))
}

pub fn get_block_by_number(index: usize) -> Block {
    rlp::decode(&get_hash_raw(BLOCK_HASHES.lock().unwrap()[index]))
}

pub fn get_total_header_by_hash(key: H256) -> TotalHeader {
    TOTAL_HEADERS.lock().unwrap().get(&key).unwrap().clone()
}

pub fn get_total_header_by_number(index: usize) -> TotalHeader {
    TOTAL_HEADERS.lock().unwrap().get(&BLOCK_HASHES.lock().unwrap()[index]).unwrap().clone()
}

pub fn get_last_256_block_hashes() -> Vec<H256> {
    let mut hashes = BLOCK_HASHES.lock().unwrap().clone();
    let mut ret = Vec::new();

    for _ in 0..256 {
        match hashes.pop() {
            Some(val) => ret.push(val),
            None => break,
        }
    }

    ret
}

pub fn current_block() -> Block {
    get_block_by_number(block_height() - 1)
}

pub fn stateful() -> MutexGuard<'static, MemoryStateful> {
    STATEFUL.lock().unwrap()
}

pub fn accounts() -> Vec<SecretKey> {
    ACCOUNTS.lock().unwrap().clone()
}

pub fn append_account(key: SecretKey) {
    ACCOUNTS.lock().unwrap().push(key)
}
