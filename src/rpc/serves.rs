use super::Error;

pub fn web3_client_version(_params: ()) -> Result<&'static str, Error> {
    Ok("sputnikvm-dev/v0.1")
}
