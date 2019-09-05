    globl	main                    ; -- Begin function main
	.p2align	1
	.type	main,@function
main:                                   ; @main
    PUSHINT $str$
    PUSHINT $persistent-base$
    ADDCONST 8
    PUSHROOT
    CTOS
    PLDDICT
    PUSHINT 64
    DICTIGET
    THROWIFNOT 100
    PLDDICT
    DUP 
    XCHG s1, s2
    PUSHINT 64
    DICTIGET
    THROWIFNOT 100
    PUSHSLICE x48656C6C6F20776F ; 'hello wo' - 8 bytes
    SDEQ
    THROWIFNOT 100

    PUSHINT $str+8$
    SWAP
    PUSHINT 64
    DICTIGET
    THROWIFNOT 100
    PUSHSLICE x726C640000000000 ; 'rld' - 3 bytes + 5 bytes 0
    SDEQ
    THROWIFNOT 100

    .globl str
    .type str, @object
str:
    .asciz	"Hello world"
    .size	str, 12