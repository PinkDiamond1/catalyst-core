use super::JormungandrRest;
use jormungandr_lib::interfaces::{AccountState, Value};
use jormungandr_testing_utils::wallet::Wallet;

pub struct JormungandrStateVerifier {
    rest: JormungandrRest,
    snapshot_before: Option<StateSnapshot>,
}

impl JormungandrStateVerifier {
    pub fn new(rest: JormungandrRest) -> Self {
        Self {
            rest,
            snapshot_before: None,
        }
    }

    pub fn record_wallets_state(mut self, wallets: Vec<&Wallet>) -> Self {
        self.snapshot_before = Some(StateSnapshot::new(
            wallets
                .iter()
                .map(|w| {
                    (
                        w.address().to_string(),
                        self.rest
                            .account_state(w)
                            .expect("cannot rerieve account state"),
                    )
                })
                .collect(),
        ));
        self
    }

    pub fn value_moved_between_wallets(
        &self,
        from: &Wallet,
        to: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        self.wallet_lost_value(&from, value.clone())?;
        self.wallet_gain_value(&to, value)?;
        Ok(())
    }

    pub fn wallet_lost_value(
        &self,
        wallet: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(wallet)?;
        let actual = self
            .rest
            .account_state(wallet)?
            .value()
            .checked_add(value)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    pub fn wallet_gain_value(
        &self,
        wallet: &Wallet,
        value: Value,
    ) -> Result<(), StateVerifierError> {
        let snapshot = self
            .snapshot_before
            .as_ref()
            .ok_or(StateVerifierError::NoSnapshot)?;
        let expected = snapshot.value_for(wallet)?.checked_add(value)?;
        let actual = self.rest.account_state(wallet)?.value().clone();
        assert_eq!(expected, actual);
        Ok(())
    }
}

use std::collections::HashMap;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum StateVerifierError {
    #[error("cannot find wallet in snapshot {0}")]
    NoWalletInSnapshot(String),
    #[error("no snapshot was made prior assert execution")]
    NoSnapshot,
    #[error("rest error")]
    RestError(#[from] crate::common::jormungandr::rest::RestError),
    #[error("wrong value calculation")]
    ValueError(#[from] chain_impl_mockchain::value::ValueError),
}

pub struct StateSnapshot {
    wallets: HashMap<String, AccountState>,
}

impl StateSnapshot {
    pub fn new(wallets: HashMap<String, AccountState>) -> Self {
        Self { wallets }
    }

    pub fn value_for(&self, wallet: &Wallet) -> Result<Value, StateVerifierError> {
        let address = wallet.address().to_string();
        let state = self
            .wallets
            .get(&address)
            .ok_or(StateVerifierError::NoWalletInSnapshot(address.clone()))?;
        Ok(state.value().clone())
    }
}
