	.text
	.file	"pbank.c"
	.globl	get_smc_first_slice
	.p2align	1
	.type	get_smc_first_slice,@function
get_smc_first_slice:
	PUSH	c5
	CTOS
	LDREF
	POP	s0
	CTOS
.Lfunc_end0:
	.size	get_smc_first_slice, .Lfunc_end0-get_smc_first_slice

	.globl	get_smc_info_remaining_balance
	.p2align	1
	.type	get_smc_info_remaining_balance,@function
get_smc_info_remaining_balance:
	PUSHINT	$get_smc_first_slice$
	CALL	1
	PUSHINT	64
	LDSLICEX
	POP	s1
	PUSHINT	32
	LDUX
	POP	s0
.Lfunc_end1:
	.size	get_smc_info_remaining_balance, .Lfunc_end1-get_smc_info_remaining_balance

	.globl	get_smc_info_block_ut
	.p2align	1
	.type	get_smc_info_block_ut,@function
get_smc_info_block_ut:
	PUSHINT	$get_smc_first_slice$
	CALL	1
	PUSHINT	96
	LDSLICEX
	POP	s1
	PUSHINT	32
	LDUX
	POP	s0
.Lfunc_end2:
	.size	get_smc_info_block_ut, .Lfunc_end2-get_smc_info_block_ut

	.globl	init_persistent_data
	.p2align	1
	.type	init_persistent_data,@function
init_persistent_data:
	PUSHINT 86400	
	PUSHINT 300
	PUSH	c4
	CTOS
	NEWC
	STSLICE
	STU	32
	STU	32
	PUSHINT	$get_smc_info_block_ut$
	CALL	1
	XCHG	s0, s1
	STU	32
	ENDC
	POPROOT
.Lfunc_end3:
	.size	init_persistent_data, .Lfunc_end3-init_persistent_data

	.globl	get_persistent_total_value
	.p2align	1
	.type	get_persistent_total_value,@function
get_persistent_total_value:
	PUSHINT	$init_persistent_data$
	CALL	1
	PUSH	c4
	CTOS
	PUSHINT	256
	LDSLICEX
	POP	s1
	PUSHINT	32
	LDUX
	POP	s0
.Lfunc_end4:
	.size	get_persistent_total_value, .Lfunc_end4-get_persistent_total_value

	.globl	make_internal_msg_cell
	.p2align	1
	.type	make_internal_msg_cell,@function
make_internal_msg_cell:
	NEWC
	PUSHINT	0
	XCHG	s0, s1
	STU	5
	PUSHINT	-1
	XCHG	s0, s1
	STI	8
	XCHG	s1, s2
	STU	256
	STU	32
	ENDC
.Lfunc_end5:
	.size	make_internal_msg_cell, .Lfunc_end5-make_internal_msg_cell

	.globl	execute_transaction
	.p2align	1
	.type	execute_transaction,@function
execute_transaction:
	PUSHINT	$make_internal_msg_cell$
	CALL	1
	PUSHINT	0
	SENDRAWMSG
.Lfunc_end6:
	.size	execute_transaction, .Lfunc_end6-execute_transaction

	.globl	transfer_authorized
	.p2align	1
	.type	transfer_authorized,@function
transfer_authorized:
	PUSHINT 0
	PUSHINT	$get_smc_info_remaining_balance$
	CALL	1
	PUSHINT	$get_persistent_total_value$
	CALL	1
	PUSH	s1
	XCHG	s0, s1
	LESS
	PUSHINT	1
	AND
	THROWIF	61
	PUSHINT	$execute_transaction$
	CALL	1
.Lfunc_end7:
	.size	transfer_authorized, .Lfunc_end7-transfer_authorized


	.ident	"clang version 7.0.0 (tags/RELEASE_700/final)"
	.section	".note.GNU-stack","",@progbits
