use crate::common::jcli::command::rest::v0::MessageCommand;
use crate::common::jcli::command::{AddressCommand, GenesisCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::FileWriteStr;
use assert_fs::{fixture::ChildPath, NamedTempFile};
use chain_addr::Discrimination;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::{crypto::hash::Hash, interfaces::FragmentLog};
use jormungandr_testing_utils::testing::process::ProcessOutput as _;
use jortestkit::prelude::ProcessOutput;
use std::str::FromStr;
use std::{collections::BTreeMap, path::Path};

pub struct Message {
    message_command: MessageCommand,
}

impl Message {
    pub fn new(message_command: MessageCommand) -> Self {
        Self { message_command }
    }

    pub fn post<S: Into<String>>(self, fragment: &str, host: S) -> FragmentId {
        let transaction_file = NamedTempFile::new("transaction.hash").unwrap();
        transaction_file.write_str(fragment).unwrap();

        self.message_command
            .post(transaction_file.path(), host.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_hash()
            .into_hash()
    }

    pub fn logs<S: Into<String>>(self, host: S) -> Vec<FragmentLog> {
        let content = self
            .message_command
            .logs(host.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();

        serde_yaml::from_str(&content).expect("Failed to parse fragment log")
    }
}
