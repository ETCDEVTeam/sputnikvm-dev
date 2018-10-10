# SputnikVM Developer Environment

[![Build Status](https://travis-ci.org/ethereumproject/sputnikvm-dev.svg?branch=master)](https://travis-ci.org/ethereumproject/sputnikvm-dev) [![Build status](https://ci.appveyor.com/api/projects/status/k4wytswph99qt9m9?svg=true)](https://ci.appveyor.com/project/splix/sputnikvm-dev)

Development environment based on SputnikVM and etcommon.

## Usage

You can either download `svmdev` from the release page, or build it by yourself by installing Rust, and run `cargo run`. We currently support Linux and MacOS, and Windows.

```
svmdev 0.2.1
Ethereum Classic Contributors
SputnikVM Development Environment, a replacement for ethereumjs-testrpc.

USAGE:
    svmdev [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --accounts <ACCOUNTS>      Additional accounts to be generated, default to 9.
    -b, --balance <BALANCE>        Balance in Wei for the account to be generated, default is 0x10000000000000000000000000000.
    -c, --chain <CHAIN>            Specify the chain to use. Refer to the documentation for a full list of valid values.
    -l, --listen <LISTEN>          Listen address and port for the RPC, e.g. 127.0.0.1:8545.
    -m, --minemode <MINE_MODE>     Specify the mining mode by number of transactions per block: [AllPending, OnePerBlock]
    -k, --private <PRIVATE_KEY>    Private key for the account to be generated, if not provided, a random private key will be generated.
```

After started, `svmdev` will print out the address and private key with balance for testing. It will then generate new blocks every ten seconds, and include all pending transactions that yet to be confirmed. You can then use the RPC endpoints below to test your blockchain application.

## Supported RPC Endpoints

Below is a list of all the supported RPC endpoints by `sputnikvm-dev`.

* [web3_clientVersion](#web3_clientversion)
* [web3_sha3](#web3_sha3)
* [net_version](#net_version)
* [net_peerCount](#net_peercount)
* [net_listening](#net_listening)
* [eth_protocolVersion](#eth_protocolversion)
* [eth_syncing](#eth_syncing)
* [eth_coinbase](#eth_coinbase)
* [eth_mining](#eth_mining)
* [eth_hashrate](#eth_hashrate)
* [eth_gasPrice](#eth_gasprice)
* [eth_accounts](#eth_accounts)
* [eth_blockNumber](#eth_blocknumber)
* [eth_getBalance](#eth_getbalance)
* [eth_getStorageAt](#eth_getstorageat)
* [eth_getTransactionCount](#eth_gettransactioncount)
* [eth_getBlockTransactionCountByHash](#eth_getblocktransactioncountbyhash)
* [eth_getBlockTransactionCountByNumber](#eth_getblocktransactioncountbynumber)
* [eth_getUncleCountByBlockHash](#eth_getunclecountbyblockhash)
* [eth_getUncleCountByBlockNumber](#eth_getunclecountbyblocknumber)
* [eth_getCode](#eth_getcode)
* [eth_sign](#eth_sign)
* [eth_sendTransaction](#eth_sendtransaction)
* [eth_sendRawTransaction](#eth_sendrawtransaction)
* [eth_call](#eth_call)
* [eth_estimateGas](#eth_estimategas)
* [eth_getBlockByHash](#eth_getblockbyhash)
* [eth_getBlockByNumber](#eth_getblockbynumber)
* [eth_getTransactionByHash](#eth_gettransactionbyhash)
* [eth_getTransactionByBlockHashAndIndex](#eth_gettransactionbyblockhashandindex)
* [eth_getTransactionByBlockNumberAndIndex](#eth_gettransactionbyblocknumberandindex)
* [eth_getTransactionReceipt](#eth_gettransactionreceipt)
* [eth_getUncleByBlockHashAndIndex](#eth_getunclebyblockhashandindex)
* [eth_getUncleByBlockNumberAndIndex](#eth_getunclebyblocknumberandindex)
* [eth_getCompilers](#eth_getcompilers)
* [eth_newFilter](#eth_newfilter)
* [eth_newBlockFilter](#eth_newblockfilter)
* [eth_newPendingTransactionFilter](#eth_newpendingtransactionfilter)
* [eth_uninstallFilter](#eth_uninstallfilter)
* [eth_getFilterChanges](#eth_getfilterchanges)
* [eth_getFilterLogs](#eth_getfilterlogs)
* [eth_getLogs](#eth_getlogs)

## Supported Debug Endpoints

* debug_dumpBlock
* debug_getBlockRlp
* debug_traceBlock
* debug_traceBlockByNumber
* debug_traceBlockByHash
* debug_traceBlockFromFile
* debug_traceTransaction
