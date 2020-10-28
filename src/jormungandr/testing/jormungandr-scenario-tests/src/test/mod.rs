pub mod comm;
pub mod features;
pub mod legacy;
pub mod network;
pub mod non_functional;
pub mod utils;

use jormungandr_lib::interfaces::FragmentStatus;

use std::time::Duration;

error_chain! {

    foreign_links {
        Interactive(jortestkit::console::InteractiveCommandError);
        IoError(std::io::Error);
        Node(crate::node::Error);
        Wallet(jormungandr_testing_utils::wallet::WalletError);
        FragmentSender(jormungandr_testing_utils::testing::FragmentSenderError);
        FragmentVerifier(jormungandr_testing_utils::testing::FragmentVerifierError);
        VerificationFailed(jormungandr_testing_utils::testing::VerificationError);
        MonitorResourcesError(jormungandr_testing_utils::testing::ConsumptionBenchmarkError);
        WalletIapyxError(iapyx::ControllerError);
        ExplorerError(jormungandr_testing_utils::testing::node::ExplorerError);
    }

    links {
        Scenario(crate::scenario::Error, crate::scenario::ErrorKind);
    }

    errors {
        SyncTimeoutOccurred(info: String, timeout: Duration) {
            description("synchronization for nodes has failed"),
            display("synchronization for nodes has failed. {}. Timeout was: {} s", info, timeout.as_secs()),
        }

        AssertionFailed(info: String) {
            description("assertion has failed"),
            display("{}", info),
        }
        TransactionNotInBlock(node: String, status: FragmentStatus) {
            description("transaction not in block"),
            display("transaction should be 'In Block'. status: {:?}, node: {}", status, node),
        }


    }
}
