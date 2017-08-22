use jsonrpc_core;
use hexutil::ParseHexError;

#[derive(Debug)]
pub enum Error {
    InvalidParams,
    HexError,
    UnsupportedTrieQuery,
}

impl From<ParseHexError> for Error {
    fn from(val: ParseHexError) -> Error {
        Error::HexError
    }
}

impl Into<jsonrpc_core::Error> for Error {
    fn into(self) -> jsonrpc_core::Error {
        jsonrpc_core::Error::invalid_request()
    }
}
