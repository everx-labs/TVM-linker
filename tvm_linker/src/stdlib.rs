use ton_block::Serializable;
use tvm::stack::SliceData;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use self::methdict::{prepare_methods, prepare_auth_method, attach_method};
use std::collections::HashMap;

// exception codes thrown by contract's code
pub const ACCESS_DENIED_EXCEPTION:   usize = 40;
pub const NOT_FOUND_EXCEPTION:       usize = 41;
pub const UNSUPPORTED_ABI_EXCEPTION: isize = 55;

pub fn build_default_dict(auth_map: &HashMap<u32, bool>) -> SliceData {
    let auth_method = prepare_auth_method(&_AUTHENTICATE, auth_map);
    let mut std_internal_dict = prepare_methods(&[
        (1u32, _PARSE_MESSAGE.to_string()),
    ]);
    std_internal_dict = attach_method(std_internal_dict, (0, auth_method));

    let std_dict = prepare_methods(&[
        (-1i32, _MAIN_EXTERNAL.to_string()),
        ( 0i32, _MAIN_INTERNAL.to_string()),
        // key 1 is a placeholder for dictionary of contract methods
        // key 2 is a placeholder for internal std methods
    ]);

    let key = 2i32.write_to_new_cell().unwrap();
    let mut std_dict = HashmapE::with_data(32, std_dict);
    std_dict.set(key.into(), std_internal_dict).unwrap();
    std_dict.get_data()
}

pub static _SELECTOR: &str = "
    ; s0 - func_id
    ; s1.. - other data
    SETCP0
    PUSHREFSLICE        ; dictionary of methods in first reference
    OVER
    ISNPOS
    PUSHCONT {          ; if func_id negative or zero - direct call to method
        PUSHINT 32
        DICTIGETJMP     ; execute method and return
    }
    PUSHCONT {          ; get dictionary with methods
        PUSHINT 32
        DICTIGET
        THROWIFNOT 52   ; no dictionary of methods
        PUSHINT 32
        DICTUGETJMP     ; execute method and return
        THROW 51
    }
    IFELSE
";

lazy_static! {
    pub static ref _MAIN_EXTERNAL: String = format!("
    ; s0 - msg body: slice
    ; s1 - msg: cell
    ; s2 - gram balance of msg: int
    ; s3 - gram balance of contract: int

        ;call signature checker (can throw exception if signature is invalid)
        PUSHINT 0 
        CALL 2      ;assume that function returns nothing
        
        ;call msg parser
        PUSH s1     ;push msg cell on top
        PUSHINT 1
        CALL 2      ;assume thar parser returns slice - dictionary with msg fields
        
        SWAP
        ;parse ABI version (1 byte) and load function id (4 bytes)
        LDU 8       ;load ABI version
        SWAP  
        THROWIF {unsupported_abi} ; only version 0 is supported now
        LDU 32      ;load func id
        SWAP
        CALL 1      ;public method call
    ",
    unsupported_abi = UNSUPPORTED_ABI_EXCEPTION,
    );    
}

//Default main for internal messages
pub static _MAIN_INTERNAL: &str = "RET";

//TODO: place a real msg parser
pub static _PARSE_MESSAGE: &str = "
    ;args: s0 - msg cell
    ;ret: s0 - msg slice
    CTOS
";

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
    DUP
    SEMPTY
    IFRET
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

pub mod methdict {
    use std::collections::HashMap;
    use ton_block::Serializable;
    use tvm::assembler::compile_code;
    use tvm::stack::dictionary::{HashmapE, HashmapType};
    use tvm::stack::{BuilderData, SliceData};

    pub fn build_hashmap<K>(pairs: &[(K, SliceData)]) -> SliceData 
    where 
        K: Default + Serializable {
        let bit_len = SliceData::from(K::default().write_to_new_cell().unwrap()).remaining_bits();
        let mut dict = HashmapE::with_bit_len(bit_len);
        for pair in pairs.iter() {
            dict.set(pair.0.write_to_new_cell().unwrap().into(), pair.1.clone()).unwrap();
        }
        dict.get_data()
    }

    pub fn prepare_methods<T>(methods: &[(T, String)]) -> SliceData
    where T: Clone + Default + Serializable {
        let method_vec: Vec<_> = 
            methods
                .iter()
                .map(|pair| (pair.0.clone(), compile_code(&pair.1).unwrap()))
                .collect(); 
        build_hashmap(&method_vec[..])
    }

    #[allow(dead_code)]
    pub fn attach_method<T>(dict: SliceData, method: (T, SliceData)) -> SliceData
    where T: Default + Serializable {
        let key_slice: SliceData = method.0.write_to_new_cell().unwrap().into();
        let bit_len = key_slice.remaining_bits();
        let mut dict = HashmapE::with_data(bit_len, dict);
        dict.set(key_slice, method.1).unwrap();
        dict.get_data()
    }

    /// Compiles authentication function to slice and builds and 
    /// attaches auth dictionary as ref0
    pub fn prepare_auth_method<K>(method: &str, map: &HashMap<K, bool>) -> SliceData 
    where 
        K: Clone + Default + Eq + Serializable + std::hash::Hash {
        let mut method = BuilderData::from(&compile_code(method).unwrap().cell());
        let key_val_vec: Vec<_> = map
            .iter()
            .map(|pair| (pair.0.clone(), pair.1.clone().write_to_new_cell().unwrap().into()))
            .collect(); 
        if key_val_vec.len() == 0 {
            BuilderData::new().into()
        } else {
            let auth_dict = build_hashmap(&key_val_vec);
            method.checked_append_reference(auth_dict.cell()).unwrap();
            method.into()
        }
    }
}