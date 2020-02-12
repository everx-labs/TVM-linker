# Tonlabs Сonsole Tool for TON

`tonlabs-cli` is a command line interface utility designed to deploy and run smart contracts, generate messages and queries in TON blockchain.
tonlabs-cli works over tonlabs SDK.

## How to build

    cargo build [--release]

## How to run

#### All platforms
cargo run [subcommand args]

#### Linux
`> cp ./target/debug/build/ton-client-rs-<hash>/out/libton_client.so ./target/debug`

`> cd ./target/debug`

`> LD_LIBRARY_PATH=.:$LD_LIBRARY_PATH ./tonlabs-cli [subcommand args]`

## How to use

By default, tonlabs-cli connects to https://net.ton.dev.

tonlabs-cli has the following subcommands for key functions:

### 1) Generate Contract Address

    tonlabs-cli genaddr [--genkey|--setkey <keyfile.json>] <tvc>

Example: `tonlabs-cli genaddr --genkey wallet_keys.json wallet.tvc`

`wallet_keys.json` file will be created with new keypair.

### 2) Deploy Smart Contract

    tonlabs-cli deploy [--sign <keyfile>] [--wc <int8>] [--abi <abifile>] <tvc> <params> 

Example: `tonlabs-cli deploy --abi wallet.abi.json --sign wallet_keys.json wallet.tvc {param1:0}`

If `--abi` or `--sign` option is omitted in parameters it must present in config file. See below.

### 3) Call a Smart Contract Method

    tonlabs-cli call [--abi <abi_file>] [--sign <keyfile>] <address> <method> <params>

Run get-method:

    tonlabs-cli run [--abi <abi_file>] <address> <method> <params>

If `--abi` or `--sign` option is omitted in parameters, it must be specified in the config file. See below for more details.

### 3) Store Parameter Values in the  Configuration File

tonlabs-cli can remember some parameter values and use it automatically in `deploy`, `call` and `run` subcommands.

    tonlabs-cli config [--url <url>] [--abi <abifile>] [--keys <keysfile>]

Example: `tonlabs-cli config --abi wallet.abi.json --keys wallet_keys.json`

After that you can omit `--abi` and `--sign` parameters in `deploy`, `call` and `run` subcommands. 

### 4) Get Account Info

    tonlabs-cli account <address>

Example: `tonlabs-cli account 0:c63a050fe333fac24750e90e4c6056c477a2526f6217b5b519853c30495882c9`

### Sample Test Sequence
Task scope: deploy a contract to TON Labs testnet at net.ton.dev.

#### 1) compile contract and get `.tvc` file and `.abi.json`. Lets name it `contract.tvc`.

#### 2) generate contract address.

    tonlabs-cli genaddr contract.tvc --genkey contract_keys.json

Save `Raw address` printed to stdout.

#### 3) Ask the testnet giver for Grams.

Note: You have to get giver address, abi and keys. 

Let's request 10 Grams to our account.

    tonlabs-cli call --abi giver.abi.json --sign giver_keys.json <giver_address> sendTransaction {"dest":"<our_address>","value":10000000000,"bounce":false}

#### 4) Get our contract state, check that it is created in blockchain and has the `Uninit` state.

    tonlabs-cli account <raw_address>

#### 5) Deploy our contract to the testnet.

    tonlabs-cli deploy --abi contract.abi.json --sign contract_keys.json contract.tvc {<constructor_arguments>}

#### 6) Check the contract state.

    tonlabs-cli account <raw_address>

The contract should be in the `Active` state.

#### 7) Use the `call` subcommand to execute contract methods in blockchain.

    tonlabs-cli call --abi contract.abi.json --sign contract_keys.json <raw_address> methodName {<method_args>}
