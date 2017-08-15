extern crate sputnikvm;
extern crate secp256k1;
extern crate rand;
extern crate sha3;
extern crate blockchain;
extern crate bigint;
extern crate rlp;
extern crate bloom;
extern crate block;
extern crate trie;

use block::{Receipt, Block, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use bloom::LogsBloom;
use secp256k1::SECP256K1;
use secp256k1::key::{PublicKey, SecretKey};
use std::time::Duration;
use std::thread;
use std::str::FromStr;
use sputnikvm::vm::{self, ValidTransaction, Patch};
use rand::os::OsRng;
use sha3::{Digest, Keccak256};

fn transit<'a>(
    current_block: &Block, transaction: ValidTransaction,
    patch: &'static Patch, state: &mut Trie<MemoryDatabaseGuard<'a>>
) -> Receipt {
    unimplemented!()
}

fn next<'a>(
    current_block: &Block, transactions: &[Transaction], receipts: &[Receipt],
    state: &Trie<MemoryDatabaseGuard<'a>>
) -> Block {
    unimplemented!()
}

fn to_valid<'a>(
    signed: Transaction, patch: &'static Patch, state: &Trie<MemoryDatabaseGuard<'a>>
) -> ValidTransaction {
    unimplemented!()
}

fn main() {
    let patch = &vm::EIP160_PATCH;

    let mut rng = OsRng::new().unwrap();
    let secret_key = SecretKey::new(&SECP256K1, &mut rng);
    let address = Address::from_secret_key(&secret_key).unwrap();
    println!("address: {:?}", address);

    let database = MemoryDatabase::new();
    let mut state = database.create_empty();

    state.insert(address.as_ref().into(), rlp::encode(&Account {
        nonce: U256::zero(),
        balance: U256::from_str("0x10000000000000000000000000000").unwrap(),
        storage_root: database.create_empty().root(),
        code_hash: H256::from(Keccak256::digest(&[]).as_slice()),
    }).to_vec());

    let mut current_block = Block {
        header: Header {
            parent_hash: H256::default(),
            ommers_hash: database.create_empty().root(),
            beneficiary: Address::default(),
            state_root: state.root(),
            transactions_root: database.create_empty().root(),
            receipts_root: database.create_empty().root(),
            logs_bloom: LogsBloom::new(),
            difficulty: U256::zero(),
            number: U256::zero(),
            gas_limit: Gas::zero(),
            gas_used: Gas::zero(),
            timestamp: 0,
            extra_data: B256::default(),
            mix_hash: H256::default(),
            nonce: H64::default(),
        },
        transactions: Vec::new(),
        ommers: Vec::new(),
    };

    loop {
        let transactions = vec![
            {
                let unsigned = UnsignedTransaction {
                    nonce: U256::zero(),
                    gas_price: Gas::zero(),
                    gas_limit: Gas::from_str("0x100000000").unwrap(),
                    action: TransactionAction::Create,
                    value: U256::zero(),
                    input: Vec::new(),
                    network_id: Some(61),
                };
                let signed = unsigned.sign(&secret_key);
                signed
            }
        ];
        let mut receipts = Vec::new();

        for transaction in transactions.clone() {
            let valid = to_valid(transaction, patch, &state);
            let receipt = transit(&current_block, valid, patch, &mut state);
            receipts.push(receipt);
        }

        let next_block = next(&current_block, transactions.as_ref(), receipts.as_ref(),
                              &state);
        current_block = next_block;

        thread::sleep(Duration::from_millis(1000));
    }
}
