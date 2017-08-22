use jsonrpc_core;
use secp256k1;
use hexutil::ParseHexError;

#[derive(Debug)]
pub enum Error {
    InvalidParams,
    HexError,
    UnsupportedTrieQuery,
    ECDSAError,
}

impl From<ParseHexError> for Error {
    fn from(val: ParseHexError) -> Error {
        Error::HexError
    }
}

impl From<secp256k1::Error> for Error {
    fn from(val: secp256k1::Error) -> Error {
        Error::ECDSAError
    }
}

impl Into<jsonrpc_core::Error> for Error {
    fn into(self) -> jsonrpc_core::Error {
        jsonrpc_core::Error::invalid_request()
    }
}
