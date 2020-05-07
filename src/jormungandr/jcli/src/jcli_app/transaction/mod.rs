mod add_account;
mod add_certificate;
mod add_input;
mod add_output;
mod add_witness;
mod auth;
mod common;
mod finalize;
mod info;
mod mk_witness;
mod new;
mod seal;
mod staging;

use self::staging::StagingKind;
use crate::jcli_app::{
    certificate,
    utils::{key_parser, output_format},
};
use chain_core::property::Serialize as _;
use chain_impl_mockchain as chain;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Transaction {
    /// create a new staging transaction. The transaction is initially
    /// empty.
    New(new::New),

    /// add UTxO input to the transaction
    AddInput(add_input::AddInput),
    /// add Account input to the transaction
    AddAccount(add_account::AddAccount),
    /// add output to the transaction
    AddOutput(add_output::AddOutput),
    /// add output to the finalized transaction
    AddWitness(add_witness::AddWitness),
    /// set a certificate to the Transaction. If there is already
    /// an extra certificate in the transaction it will be replaced
    /// with the new one.
    AddCertificate(add_certificate::AddCertificate),
    /// Lock a transaction and start adding witnesses
    Finalize(finalize::Finalize),
    /// Finalize the transaction
    Seal(seal::Seal),
    /// get the Fragment ID from the given 'sealed' transaction
    FragmentId(common::CommonTransaction),
    /// DEPRECATED: use 'data-for-witness' instead
    Id(common::CommonTransaction),
    /// get the data to sign from the given transaction
    /// (if the transaction is edited, the returned value will change)
    DataForWitness(common::CommonTransaction),
    /// display the info regarding a given transaction
    Info(info::Info),
    /// create witnesses
    MakeWitness(mk_witness::MkWitness),
    /// make auth
    Auth(auth::Auth),
    /// get the message format out of a sealed transaction
    ToMessage(common::CommonTransaction),
}

type StaticStr = &'static str;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not open staging transaction file '{path}'")]
    StagingFileOpenFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("could not read staging transaction file '{path}'")]
    StagingFileReadFailed {
        #[source]
        source: bincode::ErrorKind,
        path: PathBuf,
    },
    #[error("could not write staging transaction file '{path}'")]
    StagingFileWriteFailed {
        #[source]
        source: bincode::ErrorKind,
        path: PathBuf,
    },
    #[error("could not process secret file '{0}'")]
    SecretFileFailed(#[from] key_parser::Error),
    /*
    SecretFileReadFailed { source: std::io::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not read secret file '{}'", path.display()) }},
    SecretFileMalformed { source: chain_crypto::bech32::Error, path: PathBuf }
        = @{{ let _ = source; format_args!("could not decode secret file '{}'", path.display()) }},
        */
    #[error("could not read witness file '{path}'")]
    WitnessFileReadFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("could not write witness file '{path}'")]
    WitnessFileWriteFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("could not parse Bech32 in witness file '{path}'")]
    WitnessFileBech32Malformed {
        #[source]
        source: bech32::Error,
        path: PathBuf,
    },
    #[error("invalid Bech32 prefix in witness file, expected '{expected}', found '{actual}' in '{path}'")]
    WitnessFileBech32HrpInvalid {
        actual: String,
        expected: StaticStr,
        path: PathBuf,
    },
    #[error("failed to encode witness as bech32")]
    WitnessFileBech32EncodingFailed(#[from] bech32::Error),
    #[error("could not parse data in witness file '{path}'")]
    WitnessFileDeserializationFailed {
        #[source]
        source: chain_core::mempack::ReadError,
        path: PathBuf,
    },
    #[error("could not serialize witness data")]
    WitnessFileSerializationFailed(#[source] std::io::Error),
    #[error("could not write info file '{path}'")]
    InfoFileWriteFailed {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("formatting output failed")]
    OutputFormatFailed(#[from] output_format::Error),

    #[error("adding certificate to {kind} transaction is not valid")]
    TxKindToAddExtraInvalid { kind: StagingKind },
    #[error("adding input to {kind} transaction is not valid")]
    TxKindToAddInputInvalid { kind: StagingKind },
    #[error("adding output to {kind} transaction is not valid")]
    TxKindToAddOutputInvalid { kind: StagingKind },
    #[error("adding witness to {kind} transaction is not valid")]
    TxKindToAddWitnessInvalid { kind: StagingKind },
    #[error("sealing {kind} transaction is not valid")]
    TxKindToSealInvalid { kind: StagingKind },
    #[error("finalizing {kind} transaction is not valid")]
    TxKindToFinalizeInvalid { kind: StagingKind },
    #[error("cannot get message from transaction in {kind} state")]
    TxKindToGetMessageInvalid { kind: StagingKind },

    #[error("too many witnesses in transaction to add another: {actual}, maximum is {max}")]
    TooManyWitnessesToAddWitness { actual: usize, max: usize },
    #[error("invalid number of witnesses in transaction to seal: {actual}, should be {expected}")]
    WitnessCountToSealInvalid { actual: usize, expected: usize },
    #[error("invalid input account, this is a UTxO address")]
    AccountAddressSingle,
    #[error("invalid input account, this is a UTxO address with delegation")]
    AccountAddressGroup,
    #[error("invalid input account, this is a script address")]
    AccountAddressScript,
    #[error("transaction finalization failed")]
    TxFinalizationFailed(#[from] chain::transaction::Error),
    #[error("serialization of message to bytes failed")]
    MessageSerializationFailed(#[source] std::io::Error),
    #[error("calculation of info failed")]
    InfoCalculationFailed(#[from] chain::value::ValueError),
    #[error("expected a single account, multisig is not supported yet")]
    InfoExpectedSingleAccount,
    #[error("making account witness requires passing spending counter")]
    MakeWitnessAccountCounterMissing,
    #[error("transaction type doesn't need payload authentification")]
    TxDoesntNeedPayloadAuth,
    #[error("transaction type need payload authentification")]
    TxNeedPayloadAuth,
    #[error("No signing keys specified (use -k or --key to specify)")]
    NoSigningKeys,
    #[error("certificate error {error}")]
    CertificateError { error: certificate::Error },

    #[error("transaction has owner stake delegation, but has {inputs} inputs, should have 1")]
    TxWithOwnerStakeDelegationMultiInputs { inputs: usize },
    #[error("transaction has owner stake delegation, but has UTxO input")]
    TxWithOwnerStakeDelegationHasUtxoInput,
    #[error("transaction has owner stake delegation, but has outputs")]
    TxWithOwnerStakeDelegationHasOutputs,
}

/*
impl From<key_parser::Error> for Error {
    fn from(kp: key_parser::Error) -> Self {
        Error::SecretFileFailed { source: kp }
    }
}
*/

impl Transaction {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Transaction::New(new) => new.exec(),
            Transaction::AddInput(add_input) => add_input.exec(),
            Transaction::AddAccount(add_account) => add_account.exec(),
            Transaction::AddOutput(add_output) => add_output.exec(),
            Transaction::AddWitness(add_witness) => add_witness.exec(),
            Transaction::AddCertificate(add_certificate) => add_certificate.exec(),
            Transaction::Finalize(finalize) => finalize.exec(),
            Transaction::Seal(seal) => seal.exec(),
            Transaction::FragmentId(common) => display_fragment_id(common),
            Transaction::Id(common) => display_id(common),
            Transaction::DataForWitness(common) => display_data_for_witness(common),
            Transaction::Info(info) => info.exec(),
            Transaction::MakeWitness(mk_witness) => mk_witness.exec(),
            Transaction::Auth(auth) => auth.exec(),
            Transaction::ToMessage(common) => display_message(common),
        }
    }
}

fn display_id(common: common::CommonTransaction) -> Result<(), Error> {
    eprintln!("DEPRECATED: use 'data-for-witness' instead");
    display_data_for_witness(common)
}

fn display_data_for_witness(common: common::CommonTransaction) -> Result<(), Error> {
    let id = common.load()?.transaction_sign_data_hash();
    println!("{}", id);
    Ok(())
}

fn display_fragment_id(common: common::CommonTransaction) -> Result<(), Error> {
    let id = common.load()?.fragment()?.hash();
    println!("{}", id);
    Ok(())
}

fn display_message(common: common::CommonTransaction) -> Result<(), Error> {
    let message = common.load()?.fragment()?;
    let bytes: Vec<u8> = message
        .serialize_as_vec()
        .map_err(Error::MessageSerializationFailed)?;
    println!("{}", hex::encode(&bytes));
    Ok(())
}
