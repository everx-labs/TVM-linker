# TVM linker

This repository stores the source code for `tvm_linker` utility. It can immediately execute a smart 
contract by emulating the computing phase of transaction.

## Prerequisites

- Latest version of Rust
- Cargo tool
[Get them here](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## How to build

```bash
$ cargo update && cargo build --release
```

## How to use

`tvm_linker` has several modes of work:

 * Decoding of `.boc` messages prepared externally.
```bash
tvm_linker decode ...
```
 * Preparing an external inbound messages in `.boc` format.
```bash
tvm_linker message ...
```
 * Emulating contract execution:

Linker can emulate compute phase of blockchain transaction. It is useful for contract debugging.

```bash
tvm_linker test ...
```

### More Help
Use `tvm_linker --help` for detailed description about all options, flags and subcommands.
