use jsonrpc_core::{self, IoHandler, Params};
use jsonrpc_http_server::*;
use jsonrpc_macros::Trailing;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{self, Value};
use bigint::{U256, H256, M256, Address, Gas};
use std::net::SocketAddr;
use std::sync::mpsc::{channel, Sender, Receiver};

mod serves;
mod filter;
mod util;
mod serialize;

use error::Error;
use super::miner;
use self::serialize::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum RPCTopicFilter {
    Single(String),
    Or(Vec<String>)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCLogFilter {
    pub from_block: Option<String>,
    pub to_block: Option<String>,
    pub address: Option<String>,
    pub topics: Option<Vec<Option<RPCTopicFilter>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCLog {
    pub removed: bool,
    pub log_index: String,
    pub transaction_index: String,
    pub transaction_hash: String,
    pub block_hash: String,
    pub block_number: String,
    pub data: String,
    pub topics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCReceipt {
    pub transaction_hash: String,
    pub transaction_index: String,
    pub block_hash: String,
    pub block_number: String,
    pub cumulative_gas_used: String,
    pub gas_used: String,
    pub contract_address: Option<String>,
    pub logs: Vec<RPCLog>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCBlock {
    pub number: String,
    pub hash: String,
    pub parent_hash: String,
    pub nonce: String,
    pub sha3_uncles: String,
    pub logs_bloom: String,
    pub transactions_root: String,
    pub state_root: String,
    pub receipts_root: String,
    pub miner: String,
    pub difficulty: String,
    pub total_difficulty: String,
    pub extra_data: String,
    pub size: String,
    pub gas_limit: String,
    pub gas_used: String,
    pub timestamp: String,
    pub transactions: Either<Vec<String>, Vec<RPCTransaction>>,
    pub uncles: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCTransaction {
    pub from: String,
    pub to: Option<String>,
    pub gas: Option<String>,
    pub gas_price: Option<String>,
    pub value: Option<String>,
    #[serde(default)]
    pub data: String,
    pub nonce: Option<String>,

    pub hash: Option<String>,
    pub block_hash: Option<String>,
    pub block_number: Option<String>,
    pub transaction_index: Option<String>,
}

build_rpc_trait! {
    pub trait EthereumRPC {
		#[rpc(name = "web3_clientVersion")]
		fn client_version(&self) -> Result<String, Error>;
		#[rpc(name = "web3_sha3")]
		fn sha3(&self, Bytes) -> Result<Hex<H256>, Error>;

		#[rpc(name = "net_version")]
		fn network_id(&self) -> Result<String, Error>;
        #[rpc(name = "net_listening")]
		fn is_listening(&self) -> Result<bool, Error>;
		#[rpc(name = "net_peerCount")]
		fn peer_count(&self) -> Result<Hex<usize>, Error>;

		#[rpc(name = "eth_protocolVersion")]
		fn protocol_version(&self) -> Result<String, Error>;
		#[rpc(name = "eth_syncing")]
		fn is_syncing(&self) -> Result<bool, Error>;
        #[rpc(name = "eth_coinbase")]
		fn coinbase(&self) -> Result<Hex<Address>, Error>;
        #[rpc(name = "eth_mining")]
		fn is_mining(&self) -> Result<bool, Error>;
		#[rpc(name = "eth_hashrate")]
		fn hashrate(&self) -> Result<String, Error>;
		#[rpc(name = "eth_gasPrice")]
		fn gas_price(&self) -> Result<Hex<Gas>, Error>;
		#[rpc(name = "eth_accounts")]
		fn accounts(&self) -> Result<Vec<Hex<Address>>, Error>;
		#[rpc(name = "eth_blockNumber")]
		fn block_number(&self) -> Result<Hex<usize>, Error>;
		#[rpc(name = "eth_getBalance")]
		fn balance(&self, Hex<Address>, Trailing<String>) -> Result<Hex<U256>, Error>;
		#[rpc(name = "eth_getStorageAt")]
		fn storage_at(&self, Hex<Address>, Hex<U256>, Trailing<String>) -> Result<Hex<M256>, Error>;
        #[rpc(name = "eth_getTransactionCount")]
		fn transaction_count(&self, Hex<Address>, Trailing<String>) -> Result<Hex<usize>, Error>;
        #[rpc(name = "eth_getBlockTransactionCountByHash")]
		fn block_transaction_count_by_hash(&self, Hex<H256>) -> Result<Option<Hex<usize>>, Error>;
        #[rpc(name = "eth_getBlockTransactionCountByNumber")]
		fn block_transaction_count_by_number(&self, String) -> Result<Option<String>, Error>;
        #[rpc(name = "eth_getUncleCountByBlockHash")]
		fn block_uncles_count_by_hash(&self, String) -> Result<Option<String>, Error>;
        #[rpc(name = "eth_getUncleCountByBlockNumber")]
		fn block_uncles_count_by_number(&self, String) -> Result<Option<String>, Error>;
		#[rpc(name = "eth_getCode")]
		fn code(&self, String, Trailing<String>) -> Result<String, Error>;
        #[rpc(name = "eth_sign")]
        fn sign(&self, String, String) -> Result<String, Error>;
        #[rpc(name = "eth_sendTransaction")]
        fn send_transaction(&self, RPCTransaction) -> Result<String, Error>;
        #[rpc(name = "eth_sendRawTransaction")]
        fn send_raw_transaction(&self, String) -> Result<String, Error>;

        #[rpc(name = "eth_call")]
        fn call(&self, RPCTransaction, Trailing<String>) -> Result<String, Error>;
        #[rpc(name = "eth_estimateGas")]
        fn estimate_gas(&self, RPCTransaction, Trailing<String>) -> Result<String, Error>;

        #[rpc(name = "eth_getBlockByHash")]
        fn block_by_hash(&self, String, bool) -> Result<RPCBlock, Error>;
        #[rpc(name = "eth_getBlockByNumber")]
        fn block_by_number(&self, String, bool) -> Result<RPCBlock, Error>;
        #[rpc(name = "eth_getTransactionByHash")]
        fn transaction_by_hash(&self, String) -> Result<RPCTransaction, Error>;
        #[rpc(name = "eth_getTransactionByBlockHashAndIndex")]
        fn transaction_by_block_hash_and_index(&self, String, String) -> Result<RPCTransaction, Error>;
        #[rpc(name = "eth_getTransactionByBlockNumberAndIndex")]
        fn transaction_by_block_number_and_index(&self, String, String) -> Result<RPCTransaction, Error>;
        #[rpc(name = "eth_getTransactionReceipt")]
        fn transaction_receipt(&self, String) -> Result<Option<RPCReceipt>, Error>;
        #[rpc(name = "eth_getUncleByBlockHashAndIndex")]
        fn uncle_by_block_hash_and_index(&self, String, String) -> Result<RPCBlock, Error>;
        #[rpc(name = "eth_getUncleByBlockNumberAndIndex")]
        fn uncle_by_block_number_and_index(&self, String, String) -> Result<RPCBlock, Error>;

        #[rpc(name = "eth_getCompilers")]
        fn compilers(&self) -> Result<Vec<String>, Error>;

        #[rpc(name = "eth_newFilter")]
        fn new_filter(&self, RPCLogFilter) -> Result<String, Error>;
        #[rpc(name = "eth_newBlockFilter")]
        fn new_block_filter(&self) -> Result<String, Error>;
        #[rpc(name = "eth_newPendingTransactionFilter")]
        fn new_pending_transaction_filter(&self) -> Result<String, Error>;
        #[rpc(name = "eth_uninstallFilter")]
        fn uninstall_filter(&self, String) -> Result<bool, Error>;

        #[rpc(name = "eth_getFilterChanges")]
        fn filter_changes(&self, String) -> Result<Either<Vec<String>, Vec<RPCLog>>, Error>;
        #[rpc(name = "eth_getFilterLogs")]
        fn filter_logs(&self, String) -> Result<Vec<RPCLog>, Error>;
        #[rpc(name = "eth_getLogs")]
        fn logs(&self, RPCLogFilter) -> Result<Vec<RPCLog>, Error>;
    }
}

pub fn rpc_loop(addr: &SocketAddr, channel: Sender<bool>) {
    let rpc = serves::MinerEthereumRPC::new(channel);
    let mut io = IoHandler::default();

    io.extend_with(rpc.to_delegate());

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![
            AccessControlAllowOrigin::Any,
            AccessControlAllowOrigin::Null,
        ]))
        .start_http(addr)
        .expect("Expect to build HTTP RPC server");

    server.wait();
}
