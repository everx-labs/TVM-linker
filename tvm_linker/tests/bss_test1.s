	.globl	main
	.type	main,@function
main:
	RET

	.type	x,@object
    .bss
	.globl  x
	.p2align 3
x:
	.quad   0
	.size   x,  8

