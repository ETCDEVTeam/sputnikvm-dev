use jsonrpc_core::{self, IoHandler, Params};
use jsonrpc_http_server::*;
use jsonrpc_macros::Trailing;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{self, Value};
use bigint::H256;
use std::net::SocketAddr;

mod serves;
mod error;

pub use self::error::Error;

use super::miner;

build_rpc_trait! {
    pub trait EthereumRPC {
		#[rpc(name = "web3_clientVersion")]
		fn client_version(&self) -> Result<String, Error>;
		#[rpc(name = "web3_sha3")]
		fn sha3(&self, String) -> Result<String, Error>;

		#[rpc(name = "net_version")]
		fn network_id(&self) -> Result<String, Error>;
        #[rpc(name = "net_listening")]
		fn is_listening(&self) -> Result<bool, Error>;
		#[rpc(name = "net_peerCount")]
		fn peer_count(&self) -> Result<String, Error>;

		#[rpc(name = "eth_protocolVersion")]
		fn protocol_version(&self) -> Result<String, Error>;
		#[rpc(name = "eth_syncing")]
		fn is_syncing(&self) -> Result<bool, Error>;
        #[rpc(name = "eth_coinbase")]
		fn coinbase(&self) -> Result<String, Error>;
        #[rpc(name = "eth_mining")]
		fn is_mining(&self) -> Result<bool, Error>;
		#[rpc(name = "eth_hashrate")]
		fn hashrate(&self) -> Result<String, Error>;
		#[rpc(name = "eth_gasPrice")]
		fn gas_price(&self) -> Result<String, Error>;
		#[rpc(name = "eth_accounts")]
		fn accounts(&self) -> Result<Vec<String>, Error>;
		#[rpc(name = "eth_blockNumber")]
		fn block_number(&self) -> Result<String, Error>;
		#[rpc(name = "eth_getBalance")]
		fn balance(&self, String, Trailing<String>) -> Result<String, Error>;
		#[rpc(name = "eth_getStorageAt")]
		fn storage_at(&self, String, String, Trailing<String>) -> Result<String, Error>;
        #[rpc(name = "eth_getTransactionCount")]
		fn transaction_count(&self, String, Trailing<String>) -> Result<String, Error>;
        #[rpc(name = "eth_getBlockTransactionCountByHash")]
		fn block_transaction_count_by_hash(&self, String) -> Result<Option<String>, Error>;
        #[rpc(name = "eth_getBlockTransactionCountByNumber")]
		fn block_transaction_count_by_number(&self, String) -> Result<Option<String>, Error>;
        #[rpc(name = "eth_getUncleCountByBlockHash")]
		fn block_uncles_count_by_hash(&self, String) -> Result<Option<String>, Error>;
        #[rpc(name = "eth_getUncleCountByBlockNumber")]
		fn block_uncles_count_by_number(&self, String) -> Result<Option<String>, Error>;
		#[rpc(name = "eth_getCode")]
		fn code(&self, String, Trailing<String>) -> Result<String, Error>;
    }
}

pub fn rpc_loop(addr: &SocketAddr) {
    let rpc = serves::MinerEthereumRPC;
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
