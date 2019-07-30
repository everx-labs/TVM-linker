	.text
	.file	"comm-test.c"
	.globl	do_smth
	.p2align	1
	.type	do_smth,@function
do_smth:
	PUSH	c0
	POP	c0
	PUSHINT	0
	RET
.Lfunc_end0:
	.size	do_smth, .Lfunc_end0-do_smth

	.type	x,@object
	.comm	x,6,4
	.type	y,@object
	.comm	y,6,8
	.type	z,@object
	.comm	z,8,8
	.type	a,@object
a:
	.byte	200
	.size a, 1

	.ident	"clang version 7.0.0 (tags/RELEASE_700/final)"
	.section	".note.GNU-stack","",@progbits
