
// exception codes thrown by contract's code
pub const ACCESS_DENIED_EXCEPTION:   usize = 40;
pub const NOT_FOUND_EXCEPTION:       usize = 41;
pub const UNSUPPORTED_ABI_EXCEPTION: isize = 55;

pub static _SELECTOR: &str = "
    ; s0 - func_id i8
    ; s1.. - other data
    PUSHREFSLICE        ; dictionary of methods in first reference (what if code more than 1023 bits: 0-ref - continue of code)
    OVER
    ISNEG
    PUSHCONT {          ; if func_id negative - direct call to method
        PUSHINT 8
        DICTIGETJMP     ; execute method and return
        THROW 51
    }
    PUSHCONT {          ; get dictionary with methods
        PUSHINT 8
        DICTIGET
        THROWIFNOT 52   ; no dictionary of methods
        PUSHINT 32
        DICTUGETJMP     ; execute method and return
        THROW 51
    }
    IFELSE
";

pub static INBOUND_EXTERNAL_PARSER: &str = "
    ; s0 - msg body: slice
    ; s1 - msg header: cell
    ; s2 - gram balance of msg: int
    ; s3 - gram balance of contract: int

    ; parse body
    LDU 8       ; load version
    NIP         ; drop version
    LDU 32      ; load func id
    POP s4      ; drop gram balance of contract
    POP s2      ; drop gram balance of msg
    DROP        ; drop header
    CALL 1
";

lazy_static! {
    pub static ref _MAIN_EXTERNAL: String = format!("
    ; s0 - msg body: slice
    ; s1 - msg: cell
    ; s2 - gram balance of msg: int
    ; s3 - gram balance of contract: int

        ;call signature checker (can throw exception if signature is invalid)
        PUSHINT 0
        CALL 1      ;assume that function returns nothing
        
        ;call msg parser
        PUSH s1     ;push msg cell on top
        PUSHINT 1
        CALL 1      ;assume thar parser returns slice - dictionary with msg fields
        
        SWAP
        ;parse ABI version (1 byte) and load function id (4 bytes)
        LDU 8       ;load ABI version
        SWAP  
        THROWIF {unsupported_abi} ; only version 0 is supported now
        LDU 32      ;load func id
        SWAP
        CALL 2      ;public method call
    ",
    unsupported_abi = UNSUPPORTED_ABI_EXCEPTION,
    );
}

//Default main for internal messages
pub static _MAIN_INTERNAL: &str = "RET";

/// Signature validation function. Signature must be placed in ref0 of body slice.
/// Function assumes that auth dictionary is located in ref0 of current continuation.
lazy_static! {
    pub static ref _AUTHENTICATE: String = format!("
    ;ref0 must contains auth dictionary
    ;args: 
    ;   s0 - body slice
    ;ret: 
    ;   s0 - body slice (modified: without ref0)
    ;throws exception if ABI version is unsupported
    ;throws exception if auth flag is not found in authentication dictionary.
    ;throws exception if signature is invalid

    DUP
    PUSHINT 40  ;preload ABI ver and func_id in separate slice
    PLDSLICEX
    LDU 8       ;load ABI version
    LDU 32      ;load func id
    ENDS
    SWAP
    THROWIF {unsupported_abi_ver}
    PUSHREFSLICE
    PUSHINT 32  ;key len in auth dictionary
    DICTUGET    ;load method's flag 
    THROWIFNOT {not_found}
    PLDU 1
    PUSHCONT {{
        DUP
        SREMPTY         ; body must have reference, if not - throw exception
        THROWIF {access_denied}
        LDREFRTOS       ; detach signature slice
        OVER
        HASHSU
        SWAP    
        PUSHROOT        ; load persistent data
        CTOS            
        LDU 256         ; load public key
        DROP            ; drop remaining data slice
        CHKSIGNU
        THROWIFNOT {access_denied}        
    }}
    IF
    ",
    unsupported_abi_ver = UNSUPPORTED_ABI_EXCEPTION,
    not_found = NOT_FOUND_EXCEPTION,
    access_denied = ACCESS_DENIED_EXCEPTION,
    );
}