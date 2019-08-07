	.text
	.file	"test_matrix1.c"
	.globl	main                    ; -- Begin function main
	.p2align	1
	.type	main,@function
main:
	PUSHINT	$bg+8$
	PUSHINT	$ag$
	DUMPSTK
	EQUAL
	THROWIF 100
	DUMP	0

.Lfunc_end0:
	.size	main, .Lfunc_end0-main
	.type	bg,@object              ; @bg
	.comm	bg,128,16
	.type	ag,@object              ; @ag
	.comm	ag,128,16
