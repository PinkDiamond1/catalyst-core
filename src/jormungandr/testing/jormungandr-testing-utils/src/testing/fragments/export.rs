use super::FragmentNode;
use crate::wallet::Wallet;
use chain_impl_mockchain::fragment::{Fragment, FragmentId};
use chrono::{DateTime, Utc};
use jormungandr_lib::interfaces::Address;
use std::io::Write;
use std::{fs, path::PathBuf};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum FragmentExporterError {
    #[error("cannot create dump folder {0}")]
    CannotCreateDumpFolder(PathBuf),
    #[error("cannot create dump file {0}")]
    CannotCreateDumpFile(PathBuf),
    #[error("cannot write fragment bin to file {0}")]
    CannotWriteFragmentToDumpFile(PathBuf),
}

pub struct FragmentExporter {
    dump_folder: PathBuf,
}

impl FragmentExporter {
    pub fn new(dump_folder: PathBuf) -> Result<Self, FragmentExporterError> {
        fs::create_dir_all(dump_folder.clone())
            .map_err(|_| FragmentExporterError::CannotCreateDumpFolder(dump_folder.clone()))?;

        Ok(Self { dump_folder })
    }

    pub fn dump_to_file(
        &self,
        fragment: &Fragment,
        sender: &Wallet,
        via: &dyn FragmentNode,
    ) -> Result<(), FragmentExporterError> {
        let file_name = self.generate_file_name(fragment, sender, via);
        let file_path = self.dump_folder.join(file_name);
        let mut file = fs::File::create(&file_path)
            .map_err(|_| FragmentExporterError::CannotCreateDumpFile(file_path))?;

        file.write_all(&self.format_fragment(fragment).as_bytes())
            .map_err(|_| {
                FragmentExporterError::CannotWriteFragmentToDumpFile(self.dump_folder.clone())
            })?;

        Ok(())
    }

    fn generate_file_name(
        &self,
        fragment: &Fragment,
        sender: &Wallet,
        via: &dyn FragmentNode,
    ) -> String {
        let now: DateTime<Utc> = Utc::now();

        format!(
            "{}_{}_from_{}_to_{}.txt",
            now.format("%F_%H_%M_%S"),
            self.format_id(fragment.hash()),
            self.format_address(sender.address()),
            via.alias()
        )
    }

    fn format_fragment(&self, fragment: &Fragment) -> String {
        use chain_core::property::Serialize;

        let bytes = fragment.serialize_as_vec().unwrap();
        hex::encode(&bytes)
    }

    fn format_address(&self, address: Address) -> String {
        self.format_hash(address.to_string())
    }

    fn format_id(&self, id: FragmentId) -> String {
        self.format_hash(id.to_string())
    }

    fn format_hash(&self, hash: String) -> String {
        let start = hash.chars().next().unwrap();
        let end = hash.chars().rev().next().unwrap();
        format!("{}_{}", start, end)
    }
}
