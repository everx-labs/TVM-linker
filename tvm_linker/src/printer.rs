/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.  You may obtain a copy of the
 * License at: https://ton.dev/licenses
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use std::fmt;
use std::sync::Arc;
use tvm::block::*;
use tvm::cells_serialization::serialize_tree_of_cells;
use tvm::stack::*;

pub struct StateInitPrinter<'a> {
    pub state: &'a StateInit,
}

impl fmt::Display for StateInitPrinter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "StateInit\n split_depth: {}\n special: {}\n data: {}\n code: {}\n lib:  {}\n",
            self.state.split_depth.as_ref().map(|x| format!("{:?}", x)).unwrap_or("None".to_string()),
            self.state.special.as_ref().map(|x| format!("{:?}", x)).unwrap_or("None".to_string()),
            tree_of_cells_into_base64(self.state.data.as_ref()),
            tree_of_cells_into_base64(self.state.code.as_ref()),
            tree_of_cells_into_base64(self.state.library.as_ref()),
        )
    }    
}

fn tree_of_cells_into_base64(root_cell: Option<&Arc<CellData>>) -> String {
    match root_cell {
        Some(cell) => {
            let mut bytes = Vec::new();
            serialize_tree_of_cells(cell, &mut bytes).unwrap();
            base64::encode(&bytes)
        },
        None => "None".to_string(),
    }
}

pub struct MsgPrinter {
    pub msg: Arc<Message>,
}

impl fmt::Display for MsgPrinter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "message header\n{}init  : {}\nbody  : {}\nbody_hex: {}\n",
            print_msg_header(&self.msg.header()),
            self.msg.state_init().as_ref().map(|x| {
                format!("{}", StateInitPrinter{ state: x })
            }).unwrap_or("None".to_string()),
            match self.msg.body() {
                Some(b) => format!("{:.2}", Arc::<CellData>::from(BuilderData::from_slice(&b))),
                None => "None".to_string(),
            },
            if self.msg.body().is_some() { hex::encode(self.msg.body().unwrap().get_bytestring(0)) } else { "None".to_string() },
        )
    }    
}

fn print_msg_header(header: &CommonMsgInfo) -> String {
    match header {
        CommonMsgInfo::IntMsgInfo(header) => {
            format!("   ihr_disabled: {}\n", header.ihr_disabled) +
            &format!("   bounce      : {}\n", header.bounce) +
            &format!("   bounced     : {}\n", header.bounced) +
            &format!("   source      : {}\n", print_int_address(&header.src)) +
            &format!("   destination : {}\n", print_int_address(&header.dst)) +
            &format!("   value       : {}\n", header.value) +
            &format!("   ihr_fee     : {}\n", header.ihr_fee) +
            &format!("   fwd_fee     : {}\n", header.fwd_fee) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        },
        CommonMsgInfo::ExtInMsgInfo(header) => {
            format!("   source      : {}\n", print_ext_address(&header.src)) +
            &format!("   destination : {}\n", print_int_address(&header.dst)) +
            &format!("   import_fee  : {}\n", header.import_fee)
        },
        CommonMsgInfo::ExtOutMsgInfo(header) => {
            format!("   source      : {}\n", print_int_address(&header.src)) +
            &format!("   destination : {}\n", print_ext_address(&header.dst)) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        }
    }
}

fn print_int_address(addr: &MsgAddressInt) -> String {
    //TODO: use display method of SliceData (std.address) when it will be implemented
    match addr {
        MsgAddressInt::AddrStd(ref std) => format!("{}:{}", std.workchain_id, hex::encode(std.address.get_bytestring(0))),
        MsgAddressInt::AddrVar(ref var) => format!("{}:{}", var.workchain_id, hex::encode(var.address.get_bytestring(0))),
        MsgAddressInt::AddrNone => format!("None"),
    }
}

fn print_ext_address(addr: &MsgAddressExt) -> String {
    match addr {
        MsgAddressExt::AddrNone => "AddrNone".to_string(),
        MsgAddressExt::AddrExtern(x) => format!("{}", x)
    }
}