use rlp;

use error::Error;
use block::{Receipt, Block, TotalHeader, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use sha3::{Digest, Keccak256};
use blockchain::chain::HeaderHash;
use secp256k1::key::SecretKey;
use sputnikvm_stateful::{MemoryStateful};

use std::sync::{Mutex, MutexGuard};
use std::collections::{HashMap, HashSet};

pub struct MinerState {
    all_pending_transaction_hashes: Vec<H256>,
    pending_transaction_hashes: Vec<H256>,
    current_block: H256,
    block_hashes: Vec<H256>,
    transaction_block_hashes: HashMap<H256, H256>,

    total_header_database: HashMap<H256, TotalHeader>,
    transaction_database: HashMap<H256, Transaction>,
    block_database: HashMap<H256, Block>,
    receipt_database: HashMap<H256, Receipt>,
    address_database: HashSet<Address>,

    accounts: Vec<SecretKey>,
    database: &'static MemoryDatabase,
    stateful: MemoryStateful<'static>,
}

impl MinerState {
    pub fn new(genesis: Block, stateful: MemoryStateful<'static>) -> Self {
        let mut block_database = HashMap::new();
        let mut transaction_block_hashes = HashMap::new();
        let mut total_header_database = HashMap::new();
        let mut block_hashes = Vec::new();

        let value = rlp::encode(&genesis).to_vec();
        let hash = genesis.header.header_hash();
        block_database.insert(hash, genesis.clone());

        assert!(genesis.transactions.len() == 0);

        total_header_database.insert(hash, TotalHeader::from_genesis(genesis.header.clone()));
        block_hashes.push(hash);

        let current_block = hash;

        Self {
            database: stateful.database(),

            block_database, transaction_block_hashes, total_header_database,
            block_hashes, current_block, stateful,

            all_pending_transaction_hashes: Vec::new(),
            pending_transaction_hashes: Vec::new(),
            transaction_database: HashMap::new(),
            receipt_database: HashMap::new(),
            address_database: HashSet::new(),

            accounts: Vec::new(),
        }
    }

    pub fn append_pending_transaction(&mut self, transaction: Transaction) -> H256 {
        let value = rlp::encode(&transaction).to_vec();
        let hash = H256::from(Keccak256::digest(&value).as_slice());

        self.transaction_database.insert(hash, transaction);
        self.pending_transaction_hashes.push(hash);
        self.all_pending_transaction_hashes.push(hash);

        hash
    }

    pub fn clear_pending_transactions(&mut self) -> Vec<Transaction> {
        let transaction_hashes = {
            let ret_hashes = self.pending_transaction_hashes.clone();
            self.pending_transaction_hashes.clear();
            ret_hashes
        };

        let mut transactions = Vec::new();
        for hash in transaction_hashes {
            transactions.push(self.transaction_database.get(&hash).unwrap().clone());
        }
        transactions
    }

    pub fn all_pending_transaction_hashes(&self) -> Vec<H256> {
        self.all_pending_transaction_hashes.clone()
    }

    pub fn append_block(&mut self, block: Block) -> H256 {
        let value = rlp::encode(&block).to_vec();
        let hash = block.header.header_hash();
        self.block_database.insert(hash, block.clone());

        for transaction in &block.transactions {
            let transaction_hash = H256::from(Keccak256::digest(&rlp::encode(transaction).to_vec()).as_slice());
            self.transaction_block_hashes.insert(transaction_hash, hash);
        }

        assert!(self.block_hashes.len() > 0);
        let parent_hash = self.block_hashes[self.block_hashes.len() - 1];
        let parent = self.total_header_database.get(&parent_hash).unwrap().clone();
        self.total_header_database.insert(hash, TotalHeader::from_parent(block.header.clone(), &parent));

        self.block_hashes.push(hash);
        self.current_block = hash;

        hash
    }

    pub fn append_address(&mut self, address: Address) {
        self.address_database.insert(address);
    }

    pub fn dump_addresses(&self) -> HashSet<Address> {
        self.address_database.clone()
    }

    pub fn insert_receipt(&mut self, transaction_hash: H256, receipt: Receipt) {
        self.receipt_database.insert(transaction_hash, receipt);
    }

    pub fn block_height(&self) -> usize {
        self.block_hashes.len() - 1
    }

    pub fn get_transaction_block_hash_by_hash(&self, key: H256) -> Result<H256, Error> {
        self.transaction_block_hashes.get(&key).map(|v| v.clone()).ok_or(Error::NotFound)
    }

    pub fn get_block_by_hash(&self, key: H256) -> Result<Block, Error> {
        self.block_database.get(&key).map(|v| v.clone()).ok_or(Error::NotFound)
    }

    pub fn get_transaction_by_hash(&self, key: H256) -> Result<Transaction, Error> {
        self.transaction_database.get(&key).map(|v| v.clone()).ok_or(Error::NotFound)
    }

    pub fn get_receipt_by_transaction_hash(&self, key: H256) -> Result<Receipt, Error> {
        self.receipt_database.get(&key).map(|v| v.clone()).ok_or(Error::NotFound)
    }

    pub fn get_block_by_number(&self, index: usize) -> Block {
        self.get_block_by_hash(self.block_hashes[index]).unwrap()
    }

    pub fn get_total_header_by_hash(&self, key: H256) -> Result<TotalHeader, Error> {
        self.total_header_database.get(&key).map(|v| v.clone()).ok_or(Error::NotFound)
    }

    pub fn get_total_header_by_number(&self, index: usize) -> TotalHeader {
        self.total_header_database.get(&self.block_hashes[index]).map(|v| v.clone()).unwrap()
    }

    pub fn get_last_256_block_hashes_by_number(&self, number: usize) -> Vec<H256> {
        let mut hashes: Vec<H256> = (&self.block_hashes[0..number]).into();
        let mut ret = Vec::new();

        for _ in 0..256 {
            match hashes.pop() {
                Some(val) => ret.push(val),
                None => break,
            }
        }

        ret
    }

    pub fn get_last_256_block_hashes(&self) -> Vec<H256> {
        self.get_last_256_block_hashes_by_number(self.current_block().header.number.as_usize())
    }

    pub fn current_block(&self) -> Block {
        self.get_block_by_number(self.block_height())
    }

    pub fn stateful_mut(&mut self) -> &mut MemoryStateful<'static> {
        &mut self.stateful
    }

    pub fn stateful(&self) -> &MemoryStateful<'static> {
        &self.stateful
    }

    pub fn stateful_at(&self, root: H256) -> MemoryStateful<'static> {
        MemoryStateful::new(self.database, root)
    }

    pub fn accounts(&self) -> Vec<SecretKey> {
        self.accounts.clone()
    }

    pub fn append_account(&mut self, key: SecretKey) {
        self.accounts.push(key)
    }
}
