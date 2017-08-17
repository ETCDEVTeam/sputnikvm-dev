use rlp;
use block::{Receipt, Block, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use sha3::{Digest, Keccak256};

use std::sync::{Mutex, MutexGuard};
use std::collections::HashMap;

lazy_static! {
    static ref PENDING_TRANSACTION_HASHES: Mutex<Vec<H256>> = Mutex::new(Vec::new());
    static ref CURRENT_BLOCK: Mutex<H256> = Mutex::new(H256::default());
    static ref BLOCK_HASHES: Mutex<Vec<H256>> = Mutex::new(Vec::new());
    static ref HASH_DATABASE: Mutex<HashMap<H256, Vec<u8>>> = Mutex::new(HashMap::new());
    static ref TRIE_DATABASE: Mutex<MemoryDatabase> = Mutex::new(MemoryDatabase::new());
}

pub fn append_pending_transaction(transaction: Transaction) {
    let value = rlp::encode(&transaction).to_vec();
    let hash = H256::from(Keccak256::digest(&value).as_slice());
    insert_hash_raw(hash, value);

    PENDING_TRANSACTION_HASHES.lock().unwrap().push(hash);
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

pub fn insert_hash_raw(key: H256, value: Vec<u8>) {
    HASH_DATABASE.lock().unwrap().insert(key, value);
}

pub fn get_hash_raw(key: H256) -> Vec<u8> {
    HASH_DATABASE.lock().unwrap().get(&key).unwrap().clone()
}

pub fn append_block(block: Block) {
    let value = rlp::encode(&block).to_vec();
    let hash = H256::from(Keccak256::digest(&value).as_slice());
    insert_hash_raw(hash, value);

    BLOCK_HASHES.lock().unwrap().push(hash);
    *CURRENT_BLOCK.lock().unwrap() = hash;
}

pub fn block_height() -> usize {
    BLOCK_HASHES.lock().unwrap().len()
}

pub fn get_block_by_hash(key: H256) -> Block {
    rlp::decode(&get_hash_raw(key))
}

pub fn get_transaction_by_hash(key: H256) -> Transaction {
    rlp::decode(&get_hash_raw(key))
}

pub fn get_block_by_number(index: usize) -> Block {
    rlp::decode(&get_hash_raw(BLOCK_HASHES.lock().unwrap()[index]))
}

pub fn trie_database() -> MutexGuard<'static, MemoryDatabase> {
    TRIE_DATABASE.lock().unwrap()
}
