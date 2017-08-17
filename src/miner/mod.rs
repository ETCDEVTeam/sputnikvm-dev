use rlp;
use block::{Receipt, Block, UnsignedTransaction, Transaction, TransactionAction, Log, FromKey, Header, Account};
use trie::{MemoryDatabase, MemoryDatabaseGuard, Trie};
use bigint::{H256, M256, U256, H64, B256, Gas, Address};
use bloom::LogsBloom;
use secp256k1::SECP256K1;
use secp256k1::key::{PublicKey, SecretKey};
use std::time::Duration;
use std::thread;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sputnikvm::vm::{self, ValidTransaction, Patch, AccountCommitment, AccountState, HeaderParams, SeqTransactionVM, VM};
use sputnikvm::vm::errors::RequireError;
use rand::os::OsRng;
use sha3::{Digest, Keccak256};
use blockchain::chain::HeaderHash;

mod state;

pub use self::state::{append_pending_transaction,
                      block_height, get_block_by_hash, get_block_by_number, current_block,
                      get_transaction_by_hash, trie_database, accounts, append_account};

fn transit<'a>(
    database: &MemoryDatabase,
    current_block: &Block, transaction: ValidTransaction,
    patch: &'static Patch, state: &mut Trie<MemoryDatabaseGuard<'a>>
) -> Receipt {
    let params = HeaderParams::from(&current_block.header);

    let mut vm = SeqTransactionVM::new(transaction, params, patch);
    loop {
        match vm.fire() {
            Ok(val) => break,
            Err(RequireError::Account(address)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        vm.commit_account(AccountCommitment::Full {
                            nonce: account.nonce,
                            address: address,
                            balance: account.balance,
                            code: code,
                        });
                    },
                    None => {
                        vm.commit_account(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::AccountCode(address)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        vm.commit_account(AccountCommitment::Code {
                            address: address,
                            code: code,
                        });
                    },
                    None => {
                        vm.commit_account(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::AccountStorage(address, index)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        let storage = database.create_trie(account.storage_root);
                        let value = storage.get(rlp::encode(&index).to_vec().as_ref()).map(|v| rlp::decode::<M256>(v.as_ref())).unwrap_or(M256::zero());

                        vm.commit_account(AccountCommitment::Storage {
                            address: address,
                            index, value
                        });
                    },
                    None => {
                        vm.commit_account(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::Blockhash(number)) => {
                vm.commit_blockhash(number, state::get_block_by_number(number.as_u64() as usize).header.header_hash());
            },
        }
    }

    for account in vm.accounts() {
        match account.clone() {
            vm::Account::Full {
                nonce, address, balance, changing_storage, code
            } => {
                let changing_storage: HashMap<U256, M256> = changing_storage.into();

                let account_rlp = state.get(address.as_ref()).unwrap();
                let mut account: Account = rlp::decode(&account_rlp);

                let mut storage_trie = database.create_trie(account.storage_root);
                for (key, value) in changing_storage {
                    let mut key_raw = Vec::new();
                    let mut value_raw = Vec::new();
                    let key: U256 = key.into();
                    let value: U256 = value.into();
                    key.to_big_endian(&mut key_raw);
                    value.to_big_endian(&mut value_raw);
                    storage_trie.insert(key_raw, value_raw);
                }

                account.balance = balance;
                account.nonce = nonce;
                account.storage_root = storage_trie.root();
                assert!(account.code_hash == H256::from(Keccak256::digest(&code).as_slice()));

                state.insert(address.as_ref().into(), rlp::encode(&account).to_vec());
            },
            vm::Account::IncreaseBalance(address, value) => {
                let account_rlp = state.get(address.as_ref()).unwrap();
                let mut account: Account = rlp::decode(&account_rlp);

                account.balance = account.balance + value;
                state.insert(address.as_ref().into(), rlp::encode(&account).to_vec());
            },
            vm::Account::DecreaseBalance(address, value) => {
                let account_rlp = state.get(address.as_ref()).unwrap();
                let mut account: Account = rlp::decode(&account_rlp);

                account.balance = account.balance - value;
                state.insert(address.as_ref().into(), rlp::encode(&account).to_vec());
            },
            vm::Account::Create {
                nonce, address, balance, storage, code, exists
            } => {
                if !exists {
                    state.remove(address.as_ref());
                } else {
                    let storage: HashMap<U256, M256> = storage.into();

                    let mut storage_trie = database.create_empty();
                    for (key, value) in storage {
                        let mut key_raw = Vec::new();
                        let mut value_raw = Vec::new();
                        let key: U256 = key.into();
                        let value: U256 = value.into();
                        key.to_big_endian(&mut key_raw);
                        value.to_big_endian(&mut value_raw);
                        storage_trie.insert(key_raw, value_raw);
                    }

                    let code_hash = H256::from(Keccak256::digest(&code).as_slice());
                    state.insert(code_hash.as_ref().into(), code.clone());

                    let account = Account {
                        nonce: nonce,
                        balance: balance,
                        storage_root: storage_trie.root(),
                        code_hash
                    };

                    state.insert(address.as_ref().into(), rlp::encode(&account).to_vec());
                }
            },
        }
    }


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
        used_gas, logs, logs_bloom, state_root: state.root(),
    };

    receipt
}

fn next<'a>(
    database: &MemoryDatabase,
    current_block: &Block, transactions: &[Transaction], receipts: &[Receipt],
    beneficiary: Address, gas_limit: Gas,
    state: &mut Trie<MemoryDatabaseGuard<'a>>
) -> Block {
    // TODO: Handle block rewards.

    debug_assert!(transactions.len() == receipts.len());

    let mut transactions_trie = database.create_empty();
    let mut receipts_trie = database.create_empty();
    let mut logs_bloom = LogsBloom::new();
    let mut gas_used = Gas::zero();

    for i in 0..transactions.len() {
        transactions_trie.insert(rlp::encode(&i).to_vec(), rlp::encode(&transactions[i]).to_vec());
        receipts_trie.insert(rlp::encode(&i).to_vec(), rlp::encode(&receipts[i]).to_vec());
        logs_bloom = logs_bloom | receipts[i].logs_bloom.clone();
        gas_used = gas_used + receipts[i].used_gas.clone();
    }

    let header = Header {
        parent_hash: current_block.header.header_hash(),
        ommers_hash: database.create_empty().root(),
        beneficiary,
        state_root: state.root(),
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

fn to_valid<'a>(
    database: &MemoryDatabase,
    signed: Transaction, patch: &'static Patch, state: &Trie<MemoryDatabaseGuard<'a>>
) -> ValidTransaction {
    let mut account_state = AccountState::default();

    loop {
        match ValidTransaction::from_transaction(&signed, &account_state, patch) {
            Ok(val) => return val.unwrap(),
            Err(RequireError::Account(address)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        account_state.commit(AccountCommitment::Full {
                            nonce: account.nonce,
                            address: address,
                            balance: account.balance,
                            code: code,
                        });
                    },
                    None => {
                        account_state.commit(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::AccountCode(address)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        account_state.commit(AccountCommitment::Code {
                            address: address,
                            code: code,
                        });
                    },
                    None => {
                        account_state.commit(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::AccountStorage(address, index)) => {
                let account = state.get(address.as_ref());

                match account {
                    Some(val) => {
                        let account: Account = rlp::decode(val.as_ref());
                        let code = state.get(account.code_hash.as_ref()).unwrap_or(Vec::new());

                        let storage = database.create_trie(account.storage_root);
                        let value = storage.get(rlp::encode(&index).to_vec().as_ref()).map(|v| rlp::decode::<M256>(v.as_ref())).unwrap_or(M256::zero());

                        account_state.commit(AccountCommitment::Storage {
                            address: address,
                            index, value
                        });
                    },
                    None => {
                        account_state.commit(AccountCommitment::Nonexist(address));
                    },
                }
            },
            Err(RequireError::Blockhash(number)) => {
                panic!()
            },
        }
    }
}

pub fn mine_loop() {
    let patch = &vm::EIP160_PATCH;

    let mut rng = OsRng::new().unwrap();
    let secret_key = SecretKey::new(&SECP256K1, &mut rng);
    let address = Address::from_secret_key(&secret_key).unwrap();
    println!("address: {:?}", address);

    {
        let database = state::trie_database();
        let mut state = database.create_empty();

        state.insert(address.as_ref().into(), rlp::encode(&Account {
            nonce: U256::zero(),
            balance: U256::from_str("0x10000000000000000000000000000").unwrap(),
            storage_root: database.create_empty().root(),
            code_hash: H256::from(Keccak256::digest(&[]).as_slice()),
        }).to_vec());

        state::append_block(Block {
            header: Header {
                parent_hash: H256::default(),
                ommers_hash: database.create_empty().root(),
                beneficiary: Address::default(),
                state_root: state.root(),
                transactions_root: database.create_empty().root(),
                receipts_root: database.create_empty().root(),
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
        });
    }

    loop {
        {
            let database = state::trie_database();
            let current_block = state::current_block();
            let transactions = state::clear_pending_transactions();

            let mut state = database.create_trie(current_block.header.state_root);
            let beneficiary = Address::default();

            let mut receipts = Vec::new();

            for transaction in transactions.clone() {
                let valid = to_valid(&database, transaction, patch, &state);
                let receipt = transit(&database, &current_block, valid, patch,
                                      &mut state);
                receipts.push(receipt);
            }

            let next_block = next(&database, &current_block, transactions.as_ref(), receipts.as_ref(),
                                  beneficiary, Gas::from_str("0x10000000000000000000000").unwrap(),
                                  &mut state);
            state::append_block(next_block);

            println!("mined a new block: {:?}", state::current_block());
        }

        thread::sleep(Duration::from_millis(10000));
    }
}
