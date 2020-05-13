/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use std::fmt;
use std::sync::Arc;
use ton_block::*;
use ton_types::cells_serialization::serialize_tree_of_cells;
use ton_types::Cell;

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

fn tree_of_cells_into_base64(root_cell: Option<&Cell>) -> String {
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
            "message header\n{}init  : {}\nbody  : {}\nbody_hex: {}\nbody_base64: {}\n",
            print_msg_header(&self.msg.header()),
            self.msg.state_init().as_ref().map(|x| {
                format!("{}", StateInitPrinter{ state: x })
            }).unwrap_or("None".to_string()),
            match self.msg.body() {
                Some(slice) => format!("{:.2}", slice.into_cell()),
                None => "None".to_string(),
            },
            self.msg.body()
                .map(|b| hex::encode(b.get_bytestring(0)))
                .unwrap_or("None".to_string()),
            tree_of_cells_into_base64(
                self.msg.body()
                    .map(|slice| slice.into_cell())
                    .as_ref(),
            ),
        )
    }    
}

fn print_msg_header(header: &CommonMsgInfo) -> String {
    match header {
        CommonMsgInfo::IntMsgInfo(header) => {
            format!("   ihr_disabled: {}\n", header.ihr_disabled) +
            &format!("   bounce      : {}\n", header.bounce) +
            &format!("   bounced     : {}\n", header.bounced) +
            &format!("   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   value       : {}\n", header.value) +
            &format!("   ihr_fee     : {}\n", header.ihr_fee) +
            &format!("   fwd_fee     : {}\n", header.fwd_fee) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        },
        CommonMsgInfo::ExtInMsgInfo(header) => {
            format!( "   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   import_fee  : {}\n", header.import_fee)
        },
        CommonMsgInfo::ExtOutMsgInfo(header) => {
            format!( "   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        }
    }
}