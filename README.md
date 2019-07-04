# TVM linker

This module takes contract source code (both in llvm or sol 
output formats), links the code, adds standard loaders,
and packs into TON binary format. Also, it can immediately
execute it after preparation for debugging purposes.

## How to use

The linker has several modes of work:

### 1) Generating ready-to-deploy contract
    tvm_linker source-code-name --lib lib-file [--abi-json <abi_file>]

Here `source-code-name` - a name of tvm asm source file, `lib-file` - a name of a library file.
Linker generates `address.tvc` file, where address is a hash from initial data and code of the contract.

If contract ABI file presents it is better to use `--abi-json` option to supply contract ABI file. Function ids will be generated according to function signatues in ABI.

To generate a new keypair and put the public key to the contract, call:

	tvm_linker source-code-name --lib lib-file --genkey key_file

where `key_file` - a name of the file to store public and private keys. The linker will generate 2 files: `key_file.pub` for public key and `key_file` for private key.

To load existing keypair, call:

	tvm_linker source-code-name --setkey key_file

### 2) Decoding of boc messages, prepared somewhere else.
To use this method, call

	tvm_linker source-code-name --decode

### 3) Preparing message in boc format.

First, generate contract as described in 1). Then use `message` subcommand to create external inbound message in boc format:

	tvm_linker address message [--init] [--data] [-w]

If you want to deploy your contract, use --init option:

	tvm_linker address --message --init

Constructor message will be created with code and data loaded from file `address.tvc`. 

Aditional, you can add body to the message with option `--data':

	tvm_linker address message --data XXXX...

Instead of `XXXX...`, specify the necessary message body in hex format. 

Linker can create ABI encoded messages:

	tvm_linker address message [--init] [-w] --abi-json <json-file-with-abi> --abi-method <method-name> --abi-params <json-string-with-params> 

`<json-file-with-abi>` - path to json file with contract interface described according to ABI specification,
`<method-name>` - name of the contract method to call,
`<json-string-with-params>` - arguments of the method, declared in json like this: `{"arg_a": "0x1234", "arg_b": "x12345678"}`

By default, -1 is used as a workchain id in contract address. To use another one, use `-w` option:

	tvm_linker address message -w 0

### 4) Emulating execution of the contract:

	tvm_linker address test --body XXXX... [--sign key-file] [--trace] [--decode-c6] [--internal value] [-s source-file]

Loads contract from file by contract address `address` and emulates contract call sending external inbound message (by default) with body defined after `--body` parameter to the contract. 

`XXXX...` is a hex string. You can insert global ids by their names using `$...$` syntax:
`$name:[0len][type]$`, where `name` is a name of global object, `len` - length in chars of the label (if `len` is bigger than `name`'s length in chars than zeros will be added at the left side to fit required length), `type` can be `x` or `X` - hexademical integer  in lowercase or uppercase. 

You have to set `-s source` option when you use $...$ syntax.

Example:

	tvm_linker address test --body 00$main$ -s source //main id will be inserted as decimal string. Dont use this case, just as example
	tvm_linker address test --body 00$main:08X$ -s source
	tvm_linker address test --body 00$main:X$ -s source
	tvm_linker address test --body 00$main:x$ -s source

If `--sign` specified, the body will be signed with the private key from `key-file` file.

Use `--trace` flag to trace VM execution, stack and registers will be printed after each executed VM command.

Use `--decode-c6` to see output actions in user friendly format.

Use `--internal` to send internal message to the contract with defined nanograms in `value`.
### More Help
Use `tvm_linker --help` for detailed description about all options, flags and subcommands.

## Input format

As a temporary measure, some LLVM-assembler like input is used.
The source code should consist of functions, started by .globl keyword:

```
	.globl	x
	<code here>
```

At the end of the file a .data section may be specified.
The data from this .data section are bundled together with the code
into the init message.

```
	.globl	x
	<code here>
	.data
	00000001
```