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

use ton_types::cells_serialization::{deserialize_tree_of_cells, serialize_tree_of_cells};
use ton_types::SliceData;
use ton_types::types::AccountId;
use std::fs;
use std::io;
use std::io::prelude::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Contract {
    code: SliceData,
    persistent_data: SliceData,
}

pub trait ContractsRepository {
    fn find(&self, id: &AccountId) -> Option<Contract>;
    fn store(&self, id: &AccountId, contract: &Contract);
    fn for_each<F>(&self, worker: F) where F: FnMut(&Self, &AccountId) -> bool;
}

pub struct FileBasedContractsRepository<T>
where
    T: Fn(&AccountId) -> PathBuf,
{
    into_path: T,
}

impl Contract {
    pub fn create(code: SliceData, persistent_data: SliceData) -> Contract {
        Contract {
            code,
            persistent_data,
        }
    }
    pub fn code(&self) -> &SliceData {
        &self.code
    }

    pub fn code_mut(&mut self) -> &mut SliceData {
        &mut self.code
    }

    pub fn persistent_data(&self) -> &SliceData {
        &self.persistent_data
    }

    pub fn data_mut(&mut self) -> &mut SliceData {
        &mut self.persistent_data
    }
}

impl<T> ContractsRepository for FileBasedContractsRepository<T>
where
    T: Fn(&AccountId) -> PathBuf,
{
    fn find(&self, id: &AccountId) -> Option<Contract> {
        let contract_path = (self.into_path)(id);
        let contract_path = Path::new(&contract_path);
        match fs::File::open(&contract_path) {
            Ok(file) => {
                match zip::ZipArchive::new(file) {
                    Ok(ref mut archive) => self.load_contract(archive).ok(),
                    Err(..) => {
                        //TODO:
                        info!("Zip archive was not recognized\n");
                        None
                    }
                }
            }
            Err(..) => {
                //TODO:
                info!(
                    "File was not found at the path specified: {}\n",
                    contract_path.display()
                );
                None
            }
        }
    }

    fn for_each<F>(&self, mut worker: F) 
    where
        F: FnMut(&Self, &AccountId) -> bool,
    {
        let tmp_acc = [0; 32];
        let full_path = (self.into_path)(&AccountId::from(tmp_acc));
        let contracts_dir = full_path.parent();
        if let None = contracts_dir {
            return;
        }
        let paths = fs::read_dir(contracts_dir.unwrap());
        if paths.is_err() {
            return;
        }
        for path in paths.unwrap() {
            let contract_path = path.unwrap().path();
            if contract_path.is_dir() {
                continue;
            }
            let account_str = contract_path.file_stem()
                .map(|file_name| { file_name.to_str().unwrap_or("") })
                .unwrap_or("");
            let res = AccountId::from_str(account_str)
                .map(|acc| worker(&self, &acc));
            if let Ok(false) = res {
                return;
            }
        }
    }

    fn store(&self, id: &AccountId, contract: &Contract) {
        let contract_path = (self.into_path)(&id);
        let file = fs::File::create(&contract_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        self.save_contract(contract, &mut zip)
            .expect("Failed to save contract");
    }
}

impl<T> FileBasedContractsRepository<T>
where
    T: Fn(&AccountId) -> PathBuf,
{
    pub fn new(into_path: T) -> FileBasedContractsRepository<T> {
        FileBasedContractsRepository::<T> { into_path }
    }

    fn save_contract<W>(
        &self,
        contract: &Contract,
        destination: &mut zip::ZipWriter<W>,
    ) -> Result<(), zip::result::ZipError>
    where
        W: Write + io::Seek,
    {
        let options = zip::write::FileOptions::default();
        let mut code_buffer = Vec::new();
        let mut data_buffer = Vec::new();
        serialize_tree_of_cells(&contract.code.into_cell(), &mut code_buffer)
            .unwrap_or_else(|err| panic!("Code error: {}", err));
        serialize_tree_of_cells(&contract.persistent_data.into_cell(), &mut data_buffer)
            .unwrap_or_else(|err| panic!("Data error: {}", err));
        destination.start_file("code.cells", options)?;
        destination.write_all(code_buffer.as_slice())?;
        destination.start_file("data.cells", options)?;
        destination.write_all(data_buffer.as_slice())?;
        destination.finish()?;
        Ok(())
    }

    fn load_contract<R>(
        &self,
        contract: &mut zip::ZipArchive<R>,
    ) -> Result<Contract, zip::result::ZipError>
    where
        R: Read + io::Seek,
    {
        let code = deserialize_tree_of_cells(&mut contract.by_name("code.cells")?)
            .unwrap_or_else(|err| panic!("Code error: {}", err)).into();
        let persistent_data = deserialize_tree_of_cells(&mut contract.by_name("data.cells")?)
            .unwrap_or_else(|err| panic!("Data error: {}", err)).into();

        Ok(Contract {
            code,
            persistent_data,
        })
    }
}
