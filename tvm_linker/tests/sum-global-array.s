	.text
	.file	"sum-global-array.c"
	.globl	sum                     ; -- Begin function sum
	.p2align	1
	.type	sum,@function
sum:                                    ; @sum
; %bb.0:                                ; %entry
                                        ; implicit-def: %32
                                        ; implicit-def: %31
                                        ; implicit-def: %30
                                        ; implicit-def: %29
                                        ; implicit-def: %28
                                        ; implicit-def: %27
                                        ; implicit-def: %26
                                        ; implicit-def: %25
                                        ; implicit-def: %24
                                        ; implicit-def: %23
                                        ; implicit-def: %22
                                        ; implicit-def: %21
                                        ; implicit-def: %20
                                        ; implicit-def: %19
                                        ; implicit-def: %18
                                        ; implicit-def: %17
                                        ; implicit-def: %16
                                        ; implicit-def: %15
                                        ; implicit-def: %14
                                        ; implicit-def: %13
                                        ; implicit-def: %12
                                        ; implicit-def: %11
                                        ; implicit-def: %10
                                        ; implicit-def: %9
                                        ; implicit-def: %8
                                        ; implicit-def: %7
                                        ; implicit-def: %6
                                        ; implicit-def: %5
                                        ; implicit-def: %4
                                        ; implicit-def: %3
                                        ; implicit-def: %2
	PUSHINT	30
	CALL	$:enter$
	PUSHINT	6
	CALL	$:frameidx$
	XCHG	s0, s2
	XCHG	s1, s2
	CALL	$:store$
	PUSHINT	-2
	CALL	$:frameidx$
	XCHG	s0, s1
	CALL	$:store$
	PUSHINT	-2
	CALL	$:frameidx$
	CALL	$:load$
	PUSHINT	9
	LESS
	PUSHCONT {
		PUSHINT	6
		CALL	$:frameidx$
		CALL	$:load$
		PUSHINT	-2
		CALL	$:frameidx$
		CALL	$:load$
		PUSH	s0
		LSHIFT	3
		PUSH	s2
		XCHG	s0, s1
		ADD
		CALL	$:load$
		XCHG	s0, s1
		INC
		XCHG	s1, s2
		PUSHINT	$sum$
		CALL	1
		ADD
		PUSHINT	14
		CALL	$:frameidx$
		XCHG	s0, s1
		CALL	$:store$
		PUSHCONT {
			PUSHINT	14
			CALL	$:frameidx$
			CALL	$:load$
			PUSHINT	30
			CALL	$:leave$
		}
		JMPX
	}
	IFJMP
	PUSHCONT {
		PUSHINT	14
		CALL	$:frameidx$
		PUSHINT	0
		CALL	$:store$
		PUSHCONT {
			PUSHINT	14
			CALL	$:frameidx$
			CALL	$:load$
			PUSHINT	30
			CALL	$:leave$
		}
		JMPX
	}
	JMPX
; %bb.1:                                ; %if.then
	PUSHINT	14
	CALL	$:frameidx$
	PUSHINT	0
	CALL	$:store$
	PUSHCONT {
		PUSHINT	14
		CALL	$:frameidx$
		CALL	$:load$
		PUSHINT	30
		CALL	$:leave$
	}
	JMPX
.LBB0_2:                                ; %if.else
	PUSHINT	6
	CALL	$:frameidx$
	CALL	$:load$
	PUSHINT	-2
	CALL	$:frameidx$
	CALL	$:load$
	PUSH	s0
	LSHIFT	3
	PUSH	s2
	XCHG	s0, s1
	ADD
	CALL	$:load$
	XCHG	s0, s1
	INC
	XCHG	s1, s2
	PUSHINT	$sum$
	CALL	1
	ADD
	PUSHINT	14
	CALL	$:frameidx$
	XCHG	s0, s1
	CALL	$:store$
	PUSHCONT {
		PUSHINT	14
		CALL	$:frameidx$
		CALL	$:load$
		PUSHINT	30
		CALL	$:leave$
	}
	JMPX
.LBB0_3:                                ; %return
	PUSHINT	14
	CALL	$:frameidx$
	CALL	$:load$
	PUSHINT	30
	CALL	$:leave$
                                        ; fallthrough return
.Lfunc_end0:
	.size	sum, .Lfunc_end0-sum
                                        ; -- End function
	.globl	main                    ; -- Begin function main
	.p2align	1
	.type	main,@function
main:                                   ; @main
; %bb.0:                                ; %entry
                                        ; implicit-def: %5
                                        ; implicit-def: %4
                                        ; implicit-def: %3
                                        ; implicit-def: %2
                                        ; implicit-def: %1
                                        ; implicit-def: %0
	PUSHINT	14
	CALL	$:enter$
	PUSHINT	-2
	CALL	$:frameidx$
	PUSHINT	0
	CALL	$:store$
	PUSHINT	0
	PUSHINT	$sum$
	PUSHINT	$array$
	CALL	1
	PUSHINT	14
	CALL	$:leave$
                                        ; fallthrough return
.Lfunc_end1:
	.size	main, .Lfunc_end1-main
                                        ; -- End function
	.type	array,@object           ; @array
	.data
	.globl	array
	.p2align	4
array:
	.quad	1                       ; 0x1
	.quad	2                       ; 0x2
	.quad	3                       ; 0x3
	.quad	4                       ; 0x4
	.quad	5                       ; 0x5
	.quad	6                       ; 0x6
	.quad	7                       ; 0x7
	.quad	8                       ; 0x8
	.quad	9                       ; 0x9
	.size	array, 72


	.ident	"clang version 7.0.0 (tags/RELEASE_700/final)"
	.section	".note.GNU-stack","",@progbits
