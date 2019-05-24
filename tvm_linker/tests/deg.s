	.text
	.file	"deg.c"
	.globl	deg
	.p2align	1
	.type	deg,@function
deg:
	PUSHINT	1
	PUSH	s1
	XCHG	s0, s1
	LESS
	PUSHCONT {
		PUSHINT	1
		POP	s1
	}
	IFJMP
	PUSHCONT {
		DEC
		PUSHINT	$deg$
		CALL	1
		PUSHINT	1
		LSHIFT
		RET
	}
	JMPX
	DEC
	PUSHINT	$deg$
	CALL	1
	PUSHINT	1
	LSHIFT
	RET
.LBB0_2:
	PUSHINT	1
	POP	s1
.Lfunc_end0:
	.size	deg, .Lfunc_end0-deg

	.globl	do_deg
	.p2align	1
	.type	do_deg,@function
do_deg:
	PUSHINT	23
	PUSHINT	$deg$
	CALL	1
.Lfunc_end1:
	.size	do_deg, .Lfunc_end1-do_deg


	.ident	"clang version 7.0.0 (tags/RELEASE_700/final)"
	.section	".note.GNU-stack","",@progbits
