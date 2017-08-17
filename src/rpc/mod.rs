use jsonrpc_core::{self, IoHandler, Params};
use jsonrpc_http_server::*;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{self, Value};
use std::net::SocketAddr;

mod serves;
mod error;

pub use self::error::Error;

use super::miner;

fn wrapper<T: Serialize>(value: Result<T, Error>) -> Result<Value, jsonrpc_core::Error> {
    if value.is_err() {
        return Err(jsonrpc_core::Error::invalid_request());
    }
    let value = value.unwrap();
    let result = serde_json::to_value(value);
    match result {
        Ok(value) => Ok(value),
        Err(e) => Err(jsonrpc_core::Error::invalid_request()),
    }
}

fn parse<T>(p: Params) -> Result<T, jsonrpc_core::Error>
where
    T: DeserializeOwned,
{
    p.parse().map_err(|_| {
        jsonrpc_core::Error::parse_error()
    })
}

pub fn rpc_loop(addr: &SocketAddr) {
    let mut io = IoHandler::default();

    io.add_method("web3_clientVersion", move |p: Params| {
        wrapper(serves::web3_client_version(parse(p)?))
    });

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![
            AccessControlAllowOrigin::Any,
            AccessControlAllowOrigin::Null,
        ]))
        .start_http(addr)
        .expect("Expect to build HTTP RPC server");

    server.wait();
}
