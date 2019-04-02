# TVM linker

This module takes contract source code (both in llvm or sol 
output formats), links the code, adds standard loaders,
and packs into TON binary format. Also, it can immediately
execute it after preparation for debugging purposes.

## How to use

The linker has several modes of work:

1) Decoding of boc messages, prepared somewhere else.
To use this method, call
```tvm-linker <source-code-name> x --decode```
Here x is any string, it is a main method name placeholder.

2) Prepare messages in boc format.
The message may be either an initialization message with code,
or a message to an exisiting contract.

The command format is the same, except for --init and --data additional
agruments.

```tvm-linker <source-code-name> <main-method-name> --message```

If you are giving an init message, use --init option:

```tvm-linker <source-code-name> <main-method-name> --message --init```

If you are giving a message to an exisiting contract, use --data option:

```tvm-linker <source-code-name> <main-method-name> --message --data 0000```

Instead of 0000 in data option, specify the necessary message body in hex
format. The code is not packed into the message, however, it is necessary
to compute address of the contract.

3) Emulate execution of some code:

```tvm-linker <source-code-name> <main-method-name>```

You may also use --data key to supply additional data to the emulator.

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