# Tonlabs console tool for TON

`tonlabs-cli` - utility with command line interface for deploying and running smart contracts, generating messages and queries in TON blockchain.
tonlabs-cli works over tonlabs SDK.

## How to build

    cargo build [--release]

## How to use

By default, tonlabs-cli connects to https://net.ton.dev.

tonlabs-cli has several subcommands:

### 1) Generate contract address

    tonlabs-cli genaddr <tvc> [--genkey|--setkey <keyfile>]

### 2) Deploy smart contract

    tonlabs-cli deploy <tvc> <abi> <params> [--sign <keyfile>] [--wc <int8>]

### 3) Call smart contract method

    tonlabs-cli send <address> --abi <abi_file> --method <method_name> --params <params> [--sign <keyfile>]

Run get-method:

    tonlabs-cli run <address> --abi <abi_file> --method <method_name> --params <params>

### 3) Create configuration file with predefined parameters `(not implemented)`

    tonlabs-cli config [--url <url>] [--abi <abi_file>] [--addr <address>] [--keys <keys_file>]

### 4) Get account info `(not implemented)`

    tonlabs-cli account <address>