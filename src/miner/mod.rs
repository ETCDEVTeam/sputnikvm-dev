use rlp;
use block::{Receipt, Block, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, Database, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use bloom::LogsBloom;
use secp256k1::SECP256K1;
use secp256k1::key::{PublicKey, SecretKey};
use std::time::Duration;
use std::thread;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::mpsc::{channel, Sender, Receiver};
use sputnikvm::vm::{self, ValidTransaction, Patch, AccountCommitment, AccountState, HeaderParams, SeqTransactionVM, VM};
use sputnikvm::vm::errors::RequireError;
use rand::os::OsRng;
use sha3::{Digest, Keccak256};
use blockchain::chain::HeaderHash;
use hexutil::*;

mod state;

pub use self::state::{append_pending_transaction,
                      block_height, get_block_by_hash, get_block_by_number, current_block,
                      get_transaction_by_hash, stateful, accounts, append_account,
                      get_total_header_by_hash, get_total_header_by_number,
                      get_transaction_block_hash_by_hash, get_receipt_by_transaction_hash,
                      all_pending_transaction_hashes, get_last_256_block_hashes};

fn next<'a>(
    current_block: &Block, transactions: &[Transaction], receipts: &[Receipt],
    beneficiary: Address, gas_limit: Gas, state_root: H256,
) -> Block {
    // TODO: Handle block rewards.

    debug_assert!(transactions.len() == receipts.len());

    let mut transactions_trie = Trie::empty(HashMap::new());
    let mut receipts_trie = Trie::empty(HashMap::new());
    let mut logs_bloom = LogsBloom::new();
    let mut gas_used = Gas::zero();

    for i in 0..transactions.len() {
        let transaction_rlp = rlp::encode(&transactions[i]).to_vec();
        let transaction_hash = H256::from(Keccak256::digest(&transaction_rlp).as_slice());
        let receipt_rlp = rlp::encode(&receipts[i]).to_vec();
        let receipt_hash = H256::from(Keccak256::digest(&receipt_rlp).as_slice());

        transactions_trie.insert(rlp::encode(&i).to_vec(), transaction_rlp.clone());
        receipts_trie.insert(rlp::encode(&i).to_vec(), receipt_rlp.clone());

        state::insert_receipt(transaction_hash, receipts[i].clone());

        logs_bloom = logs_bloom | receipts[i].logs_bloom.clone();
        gas_used = gas_used + receipts[i].used_gas.clone();
    }

    let header = Header {
        parent_hash: current_block.header.header_hash(),
        // TODO: use the known-good result from etclient
        ommers_hash: MemoryDatabase::default().create_empty().root(),
        beneficiary,
        state_root: state_root,
        // TODO: use the known-good result from etclient
        transactions_root: transactions_trie.root(),
        receipts_root: receipts_trie.root(),
        logs_bloom,
        gas_limit,
        gas_used,
        timestamp: current_timestamp(),
        extra_data: B256::default(),
        number: current_block.header.number + U256::one(),

        difficulty: U256::zero(),
        mix_hash: H256::default(),
        nonce: H64::default(),
    };

    Block {
        header,
        transactions: transactions.into(),
        ommers: Vec::new()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub fn mine_loop(secret_key: SecretKey, balance: U256, channel: Receiver<bool>) {
    let patch = &vm::EIP160_PATCH;

    let address = Address::from_secret_key(&secret_key).unwrap();
    println!("address: {:?}", address);
    println!("private key: {}", to_hex(&secret_key[..]));

    state::append_account(secret_key);

    {
        let mut stateful = state::stateful();

        let genesis = Block {
            header: Header {
                parent_hash: H256::default(),
                // TODO: use the known good result from etclient
                ommers_hash: MemoryDatabase::default().create_empty().root(),
                beneficiary: Address::default(),
                state_root: stateful.root(),
                transactions_root: MemoryDatabase::default().create_empty().root(),
                receipts_root: MemoryDatabase::default().create_empty().root(),
                logs_bloom: LogsBloom::new(),
                number: U256::zero(),
                gas_limit: Gas::zero(),
                gas_used: Gas::zero(),
                timestamp: current_timestamp(),
                extra_data: B256::default(),

                difficulty: U256::zero(),
                mix_hash: H256::default(),
                nonce: H64::default(),
            },
            transactions: Vec::new(),
            ommers: Vec::new(),
        };

        let _: SeqTransactionVM = stateful.execute(ValidTransaction {
            caller: None,
            gas_price: Gas::zero(),
            gas_limit: Gas::from(100000usize),
            action: TransactionAction::Call(address),
            value: balance,
            input: Vec::new(),
            nonce: U256::zero(),
        }, HeaderParams::from(&genesis.header), patch, &[]);

        state::append_block(genesis);
    }

    loop {
        mine_one(address, patch);

        channel.recv_timeout(Duration::new(10, 0));
    }
}

pub fn mine_one(address: Address, patch: &'static Patch) {
    let mut stateful = state::stateful();

    let current_block = state::current_block();
    let transactions = state::clear_pending_transactions();
    let block_hashes = state::get_last_256_block_hashes();

    let beneficiary = address;

    let mut receipts = Vec::new();

    for transaction in transactions.clone() {
        let valid = stateful.to_valid(transaction, patch).unwrap();
        let vm: SeqTransactionVM = stateful.execute(
            valid, HeaderParams::from(&current_block.header),
            patch, &block_hashes);

        let logs: Vec<Log> = vm.logs().into();
        let used_gas = vm.real_used_gas();
        let mut logs_bloom = LogsBloom::new();
        for log in logs.clone() {
            logs_bloom.set(&log.address);
            for topic in log.topics {
                logs_bloom.set(&topic)
            }
        }

        let receipt = Receipt {
            used_gas: used_gas.clone(),
            logs,
            logs_bloom: logs_bloom.clone(),
            state_root: stateful.root(),
        };
        receipts.push(receipt);
    }

    let next_block = next(&current_block, transactions.as_ref(), receipts.as_ref(),
                          beneficiary, Gas::from_str("0x10000000000000000000000").unwrap(),
                          stateful.root());
    state::append_block(next_block);
}
