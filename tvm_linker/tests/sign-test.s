	.globl	main
	.type	main,@function
main:
	PUSHINT $tvm_sender_pubkey$
    CALL 1
    LDU 256
    ENDS
    DUP
    THROWIFNOT 100
    DUMP 0
	RET