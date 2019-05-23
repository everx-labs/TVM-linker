## How to run

### deg.s

To find out the method id use:

	./target/debug/tvm_linker --lib stdlib.tvm ./tests/deg.s --debug

To run test use the following:
	
	./target/debug/tvm_linker --lib stdlib.tvm ./tests/deg.s test --body 0085311F81 [--trace] [--decode-c6]

Use `--decode-c6` to see output actions in user friendly format.

### pbank.s

To link with new keypair use:

	./target/debug/tvm_linker --lib stdlib.tvm ./tests/pbank.s --genkey key1

To sign body for transfer call use:

	./target/debug/tvm_linker --lib stdlib.tvm ./tests/pbank.s --genkey key1
	./target/debug/tvm_linker --lib stdlib.tvm ./tests/pbank.s --setkey key1 test --body 00XXXXXXXX --sign key1 

where XXXXXXXX - id of transfer method (6E118F89).

