    globl	main                    ; -- Begin function main
	.p2align	1
	.type	main,@function
    .public main
main:                                   ; @main
    PUSHINT $str$
    PUSHINT $persistent-base$           ; { str | persistent-base | - }
    ADDCONST 8                          ; { str | persistent-base + 8 | - }
    PUSHROOT                            ; { str | persistent-base + 8 | root | - }
    CTOS                                ; { str | persistent-base + 8 | root-slice | - }
    PLDDICT                             ; { str | persistent-base + 8 | root-slice-dict | - }
    PUSHINT 64                          ; { str | persistent-base + 8 | root-slice-dict | 64 | - }
    DICTIGET                            ; { str | root-slice-dict->persistent-base[1] | Succ | - }
    THROWIFNOT 101                      ; { str | root-slice-dict->persistent-base[1] | - }
    PLDDICT                             ; { str | persistent-base[1]-dict=dict | - }
    DUP                                 ; { str | dict | dict | - }
    XCHG s1, s2                         ; { dict | str | dict | - }
    PUSHINT 64                          ; { dict | str | dict | 64 | - }
    DICTIGET                            ; { dict | str[0].slice | Succ | - }
    THROWIFNOT 102                      ; { dict | str[0].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[0] | - }
    PUSHINT 72 ; 'H'                    ; { dict | str[0] | 'H' | - }
    EQUAL                               ; { dict | str[0] == 'H' | - }
    THROWIFNOT 103                      ; { dict | - }
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+1$                     ; { dict | dict | str+1 | - }
    SWAP                                ; { dict | str+1 | dict | - }
    PUSHINT 64                          ; { dict | str+1 | dict | 64 | - }
    DICTIGET                            ; { dict | str[1].slice | Succ | - }
    THROWIFNOT 104                      ; { dict | str[1].slice | - }
    PUSHINT 257 LDIX
    ENDS
    PUSHINT 101 ; 'e'                   ; { dict | str[1] | 'e' | - }
    EQUAL                               ; { dict | str[1] == 'e' | - }
    THROWIFNOT 105                      ; { dict | - }
    
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+2$                     ; { dict | dict | str+2 | - }
    SWAP                                ; { dict | str+2 | dict | - }
    PUSHINT 64                          ; { dict | str+2 | dict | 64 | - }
    DICTIGET                            ; { dict | str[2].slice | Succ | - }
    THROWIFNOT 106                      ; { dict | str[2].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[2] | - }
    PUSHINT 108 ; 'l'                   ; { dict | str[2] | 'l' | - }
    EQUAL                               ; { dict | str[2] == 'l' | - }
    THROWIFNOT 107                      ; { dict | - }
    
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+3$                     ; { dict | dict | str+3 | - }
    SWAP                                ; { dict | str+3 | dict | - }
    PUSHINT 64                          ; { dict | str+3 | dict | 64 | - }
    DICTIGET                            ; { dict | str[3].slice | Succ | - }
    THROWIFNOT 108                      ; { dict | str[3].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[3] | - }
    PUSHINT 108 ; 'l'                   ; { dict | str[3] | 'l' | - }
    EQUAL                               ; { dict | str[3] == 'l' | - }
    THROWIFNOT 109                      ; { dict | - }

    DUP                                 ; { dict | dict | - }
    PUSHINT $str+4$                     ; { dict | dict | str+4 | - }
    SWAP                                ; { dict | str+4 | dict | - }
    PUSHINT 64                          ; { dict | str+4 | dict | 64 | - }
    DICTIGET                            ; { dict | str[4].slice | Succ | - }
    THROWIFNOT 110                      ; { dict | str[4].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[4] | - }
    PUSHINT 111 ; 'o'                   ; { dict | str[4] | 'l' | - }
    EQUAL                               ; { dict | str[4] == 'l' | - }
    THROWIFNOT 111                      ; { dict | - }
 
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+5$                     ; { dict | dict | str+5 | - }
    SWAP                                ; { dict | str+5 | dict | - }
    PUSHINT 64                          ; { dict | str+5 | dict | 64 | - }
    DICTIGET                            ; { dict | str[5].slice | Succ | - }
    THROWIFNOT 112                      ; { dict | str[5].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[5] | - }
    PUSHINT 32 ; ' '                    ; { dict | str[5] | 'l' | - }
    EQUAL                               ; { dict | str[5] == 'l' | - }
    THROWIFNOT 113                      ; { dict | - }

    DUP                                 ; { dict | dict | - }
    PUSHINT $str+6$                     ; { dict | dict | str+6 | - }
    SWAP                                ; { dict | str+6 | dict | - }
    PUSHINT 64                          ; { dict | str+6 | dict | 64 | - }
    DICTIGET                            ; { dict | str[6].slice | Succ | - }
    THROWIFNOT 114                      ; { dict | str[6].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[6] | - }
    PUSHINT 119 ; 'w'                   ; { dict | str[6] | 'l' | - }
    EQUAL                               ; { dict | str[6] == 'l' | - }
    THROWIFNOT 115                      ; { dict | - }

    DUP                                 ; { dict | dict | - }
    PUSHINT $str+7$                     ; { dict | dict | str+7 | - }
    SWAP                                ; { dict | str+7 | dict | - }
    PUSHINT 64                          ; { dict | str+7 | dict | 64 | - }
    DICTIGET                            ; { dict | str[7].slice | Succ | - }
    THROWIFNOT 116                      ; { dict | str[7].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[7] | - }
    PUSHINT 111 ; 'o'                   ; { dict | str[7] | 'l' | - }
    EQUAL                               ; { dict | str[7] == 'l' | - }
    THROWIFNOT 117                      ; { dict | - }

    DUP                                 ; { dict | dict | - }
    PUSHINT $str+8$                     ; { dict | dict | str+8 | - }
    SWAP                                ; { dict | str+8 | dict | - }
    PUSHINT 64                          ; { dict | str+8 | dict | 64 | - }
    DICTIGET                            ; { dict | str[8].slice | Succ | - }
    THROWIFNOT 118                      ; { dict | str[8].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[8] | - }
    PUSHINT 114 ; 'r'                   ; { dict | str[8] | 'l' | - }
    EQUAL                               ; { dict | str[8] == 'l' | - }
    THROWIFNOT 119                      ; { dict | - }
    
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+9$                     ; { dict | dict | str+9 | - }
    SWAP                                ; { dict | str+9 | dict | - }
    PUSHINT 64                          ; { dict | str+9 | dict | 64 | - }
    DICTIGET                            ; { dict | str[9].slice | Succ | - }
    THROWIFNOT 120                      ; { dict | str[9].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[9] | - }
    PUSHINT 108 ; 'l'                   ; { dict | str[9] | 'l' | - }
    EQUAL                               ; { dict | str[9] == 'l' | - }
    THROWIFNOT 121                      ; { dict | - }
    
    DUP                                 ; { dict | dict | - }
    PUSHINT $str+10$                    ; { dict | dict | str+10 | - }
    SWAP                                ; { dict | str+10 | dict | - }
    PUSHINT 64                          ; { dict | str+10 | dict | 64 | - }
    DICTIGET                            ; { dict | str[10].slice | Succ | - }
    THROWIFNOT 122                      ; { dict | str[10].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[10] | - }
    PUSHINT 100 ; 'd'                   ; { dict | str[10] | 'l' | - }
    EQUAL                               ; { dict | str[10] == 'l' | - }
    THROWIFNOT 123                      ; { dict | - }

    DUP                                 ; { dict | dict | - }
    PUSHINT $str+11$                    ; { dict | dict | str+11 | - }
    SWAP                                ; { dict | str+11 | dict | - }
    PUSHINT 64                          ; { dict | str+11 | dict | 64 | - }
    DICTIGET                            ; { dict | str[11].slice | Succ | - }
    THROWIFNOT 124                      ; { dict | str[11].slice | - }
    PUSHINT 257 LDIX
    ENDS                                ; { dict | str[11] | - }
    ZERO        ; '\0'                  ; { dict | str[11] | 'l' | - }
    EQUAL                               ; { dict | str[11] == 'l' | - }
    THROWIFNOT 125                      ; { dict | - }
    DROP                                ; { - }

    .globl str
    .type str, @object
str:
    .asciz	"Hello world"
    .size	str, 12
