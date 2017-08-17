use jsonrpc_core;
use hexutil::ParseHexError;

#[derive(Debug)]
pub enum Error {
    InvalidParams,
    HexError,
}

impl From<ParseHexError> for Error {
    fn from(val: ParseHexError) -> Error {
        Error::HexError
    }
}
