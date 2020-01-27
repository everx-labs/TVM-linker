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

### Test sequence
Task: deploy our contract to tonlabs testnet: net.ton.dev.

#### 1) compile contract and get `.tvc` file and `.abi.json`. Lets name it `contract.tvc`.

#### 2) generate contract address.

    tonlabs-cli genaddr contract.tvc --genkey contract_keys.json

Save `Raw address` printed to stdout.

#### 3) Ask testnet giver to transfer some grams to our address.

Remark: you have to get giver address, abi and keys. 

Lets ask 10 grams.

    tonlabs-cli call --abi giver.abi.json --sign giver_keys.json <giver_address> sendTransaction {"dest":"<our_address>","value":10000000000,"bounce":false}

#### 4) Get our contract state and check that it is created in blockchain and has `Uninit` state.

    tonlabs-cli account <raw_address>

#### 5) Deploy our contract to testnet.

    tonlabs-cli deploy --abi contract.abi.json --sign contract_keys.json contract.tvc {<constructor_arguments>}

#### 6) Check contract state.

    tonlabs-cli account <raw_address>

State should be `Active`.

#### 7) Use `call` subcommand to execute contract methods in blockchain.

    tonlabs-cli call --abi contract.abi.json --sign contract_keys.json <raw_address> methodName {<method_args>}

## List of abailable subcommands

By default, tonlabs-cli connects to https://net.ton.dev.

tonlabs-cli has several subcommands:

### * Generate contract address

    tonlabs-cli genaddr [--genkey|--setkey <keyfile.json>] <tvc>

Example: `tonlabs-cli genaddr --genkey wallet_keys.json wallet.tvc`

`wallet_keys.json` file will be created with new keypair.

### * Deploy smart contract

    tonlabs-cli deploy [--sign <keyfile>] [--wc <int8>] [--abi <abifile>] <tvc> <params> 

Example: `tonlabs-cli deploy --abi wallet.abi.json --sign wallet_keys.json wallet.tvc {param1:0}`

If `--abi` or `--sign` option is omitted in parameters it must present in config file. See below.

### * Call smart contract method

    tonlabs-cli call [--abi <abi_file>] [--sign <keyfile>] <address> <method> <params>

Run get-method:

    tonlabs-cli run [--abi <abi_file>] <address> <method> <params>

If `--abi` or `--sign` option is omitted in parameters it must present in config file. See below.

### * Store parameter values in configuration file

tonlabs-cli can remember some parameter values and use it automatically in `deploy`, `call` and `run` subcommands.

    tonlabs-cli config [--url <url>] [--abi <abifile>] [--keys <keysfile>]

Example: `tonlabs-cli config --abi wallet.abi.json --keys wallet_keys.json`

After that you can omit `--abi` and `--sign` parameters in `deploy`, `call` and `run` subcommands. 

### * Get account info

    tonlabs-cli account <address>

Example: `tonlabs-cli account 0:c63a050fe333fac24750e90e4c6056c477a2526f6217b5b519853c30495882c9`

## Troubleshooting

Possible issues are currently described in a separate document https://www.notion.so/tonlabs/How-to-debug-a-contract-in-test-net-5f5ad45ac26c45099e97351238991d4c
