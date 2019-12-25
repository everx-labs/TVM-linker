# Tonlabs console tool for TON

`tonlabs-cli` - utility with command line interface for deploying and running smart contracts, generating messages and queries in TON blockchain.
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

tonlabs-cli has several subcommands:

### 1) Generate contract address

    tonlabs-cli genaddr [--genkey|--setkey <keyfile.json>] <tvc>

Example: `tonlabs-cli genaddr --genkey wallet_keys.json wallet.tvc`

`wallet_keys.json` file will be created with new keypair.

### 2) Deploy smart contract

    tonlabs-cli deploy [--sign <keyfile>] [--wc <int8>] [--abi <abifile>] <tvc> <params> 

Example: `tonlabs-cli deploy --abi wallet.abi.json --sign wallet_keys.json wallet.tvc {param1:0}`

If `--abi` or `--sign` option is omitted in parameters it must present in config file. See below.

### 3) Call smart contract method

    tonlabs-cli call [--abi <abi_file>] [--sign <keyfile>] <address> <method> <params>

Run get-method:

    tonlabs-cli run [--abi <abi_file>] <address> <method> <params>

If `--abi` or `--sign` option is omitted in parameters it must present in config file. See below.

### 3) Store parameter values in configuration file

tonlabs-cli can remember some parameter values and use it automatically in `deploy`, `call` and `run` subcommands.

    tonlabs-cli config [--url <url>] [--abi <abifile>] [--keys <keysfile>]

Example: `tonlabs-cli config --abi wallet.abi.json --keys wallet_keys.json`

After that you can omit `--abi` and `--sign` parameters in `deploy`, `call` and `run` subcommands. 

### 4) Get account info `(not implemented)`

    tonlabs-cli account <address>