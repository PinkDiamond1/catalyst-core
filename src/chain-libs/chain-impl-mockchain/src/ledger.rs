//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::block::{BlockDate, ChainLength, ConsensusVersion, HeaderHash};
use crate::config::{self, ConfigParam};
use crate::fee::{FeeAlgorithm, LinearFee};
use crate::leadership::genesis::ActiveSlotsCoeffError;
use crate::message::Message;
use crate::stake::{CertificateApplyOutput, DelegationError, DelegationState, StakeDistribution};
use crate::transaction::*;
use crate::value::*;
use crate::{account, certificate, legacy, multisig, setting, stake, update, utxo};
use chain_addr::{Address, Discrimination, Kind};
use chain_core::property::{self, ChainLength as _, Message as _};
use chain_time::{Epoch, SlotDuration, TimeEra, TimeFrame, Timeline};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// static parameters, effectively this is constant in the parameter of the blockchain
#[derive(Clone)]
pub struct LedgerStaticParameters {
    pub block0_initial_hash: HeaderHash,
    pub block0_start_time: config::Block0Date,
    pub discrimination: Discrimination,
}

// parameters to validate ledger
#[derive(Clone)]
pub struct LedgerParameters {
    pub fees: LinearFee,
    pub allow_account_creation: bool,
}

/// Overall ledger structure.
///
/// This represent a given state related to utxo/old utxo/accounts/... at a given
/// point in time.
///
/// The ledger can be easily and cheaply cloned despite containing reference
/// to a lot of data (millions of utxos, thousands of accounts, ..)
#[derive(Clone)]
pub struct Ledger {
    pub(crate) utxos: utxo::Ledger<Address>,
    pub(crate) oldutxos: utxo::Ledger<legacy::OldAddress>,
    pub(crate) accounts: account::Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) updates: update::UpdateState,
    pub(crate) multisig: multisig::Ledger,
    pub(crate) delegation: DelegationState,
    pub(crate) static_params: Arc<LedgerStaticParameters>,
    pub(crate) date: BlockDate,
    pub(crate) chain_length: ChainLength,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block0Error {
    OnlyMessageReceived,
    TransactionHasInput,
    TransactionHasOutput,
    TransactionHasWitnesses,
    InitialMessageMissing,
    InitialMessageMany,
    InitialMessageDuplicateBlock0Date,
    InitialMessageDuplicateDiscrimination,
    InitialMessageDuplicateConsensusVersion,
    InitialMessageDuplicateSlotDuration,
    InitialMessageDuplicateEpochStabilityDepth,
    InitialMessageDuplicatePraosActiveSlotsCoeff,
    InitialMessageNoDate,
    InitialMessageNoSlotDuration,
    InitialMessageNoSlotsPerEpoch,
    InitialMessageNoDiscrimination,
    InitialMessageNoConsensusVersion,
    InitialMessageNoConsensusLeaderId,
    InitialMessageNoPraosActiveSlotsCoeff,
    UtxoTotalValueTooBig,
    HasUpdateProposal,
    HasUpdateVote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Config(config::Error),
    NotEnoughSignatures(usize, usize),
    UtxoValueNotMatching(Value, Value),
    UtxoError(utxo::Error),
    UtxoInvalidSignature(UtxoPointer, Output<Address>, Witness),
    OldUtxoInvalidSignature(UtxoPointer, Output<legacy::OldAddress>, Witness),
    OldUtxoInvalidPublicKey(UtxoPointer, Output<legacy::OldAddress>, Witness),
    AccountInvalidSignature(account::Identifier, Witness),
    MultisigInvalidSignature(multisig::Identifier, Witness),
    TransactionHasNoInput,
    FeeCalculationError(ValueError),
    PraosActiveSlotsCoeffInvalid(ActiveSlotsCoeffError),
    UtxoInputsTotal(ValueError),
    UtxoOutputsTotal(ValueError),
    Block0(Block0Error),
    Account(account::LedgerError),
    Multisig(multisig::LedgerError),
    NotBalanced(Value, Value),
    ZeroOutput(Output<Address>),
    Delegation(DelegationError),
    AccountIdentifierInvalid,
    InvalidDiscrimination,
    ExpectingAccountWitness,
    ExpectingUtxoWitness,
    ExpectingInitialMessage,
    CertificateInvalidSignature,
    Update(update::Error),
    WrongChainLength {
        actual: ChainLength,
        expected: ChainLength,
    },
    NonMonotonicDate {
        block_date: BlockDate,
        chain_date: BlockDate,
    },
}

impl From<utxo::Error> for Error {
    fn from(e: utxo::Error) -> Self {
        Error::UtxoError(e)
    }
}

impl From<Block0Error> for Error {
    fn from(e: Block0Error) -> Self {
        Error::Block0(e)
    }
}

impl From<account::LedgerError> for Error {
    fn from(e: account::LedgerError) -> Self {
        Error::Account(e)
    }
}

impl From<multisig::LedgerError> for Error {
    fn from(e: multisig::LedgerError) -> Self {
        Error::Multisig(e)
    }
}

impl From<DelegationError> for Error {
    fn from(e: DelegationError) -> Self {
        Error::Delegation(e)
    }
}

impl From<config::Error> for Error {
    fn from(e: config::Error) -> Self {
        Error::Config(e)
    }
}

impl From<update::Error> for Error {
    fn from(e: update::Error) -> Self {
        Error::Update(e)
    }
}

impl Ledger {
    fn empty(settings: setting::Settings, static_params: LedgerStaticParameters) -> Self {
        Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
            settings,
            updates: update::UpdateState::new(),
            multisig: multisig::Ledger::new(),
            delegation: DelegationState::new(),
            static_params: Arc::new(static_params),
            date: BlockDate::first(),
            chain_length: ChainLength(0),
        }
    }

    pub fn new<'a, I>(block0_initial_hash: HeaderHash, contents: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Message>,
    {
        let mut content_iter = contents.into_iter();

        let init_ents = match content_iter.next() {
            Some(Message::Initial(ref init_ents)) => Ok(init_ents),
            Some(_) => Err(Error::ExpectingInitialMessage),
            None => Err(Error::Block0(Block0Error::InitialMessageMissing)),
        }?;

        let mut regular_ents = crate::message::ConfigParams::new();
        let mut block0_start_time = None;
        let mut slot_duration = None;
        let mut discrimination = None;
        let mut slots_per_epoch = None;

        for param in init_ents.iter() {
            match param {
                ConfigParam::Block0Date(d) => {
                    block0_start_time = Some(*d);
                }
                ConfigParam::Discrimination(d) => {
                    discrimination = Some(*d);
                }
                ConfigParam::SlotDuration(d) => {
                    slot_duration = Some(*d);
                }
                ConfigParam::SlotsPerEpoch(n) => {
                    slots_per_epoch = Some(*n);
                }
                _ => regular_ents.push(param.clone()),
            }
        }

        // here we make sure those specific parameters are present, otherwise we returns a given error
        let block0_start_time =
            block0_start_time.ok_or(Error::Block0(Block0Error::InitialMessageNoDate))?;
        let discrimination =
            discrimination.ok_or(Error::Block0(Block0Error::InitialMessageNoDiscrimination))?;
        let slot_duration =
            slot_duration.ok_or(Error::Block0(Block0Error::InitialMessageNoSlotDuration))?;
        let slots_per_epoch =
            slots_per_epoch.ok_or(Error::Block0(Block0Error::InitialMessageNoSlotsPerEpoch))?;

        let static_params = LedgerStaticParameters {
            block0_initial_hash,
            block0_start_time: block0_start_time,
            discrimination: discrimination,
        };

        let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(block0_start_time.0);
        let timeline = Timeline::new(system_time);
        let tf = TimeFrame::new(timeline, SlotDuration::from_secs(slot_duration as u32));
        let slot0 = tf.slot0();

        let era = TimeEra::new(slot0, Epoch(0), slots_per_epoch);

        let settings = setting::Settings::new(era).apply(&regular_ents)?;

        if settings.bft_leaders.is_empty() {
            return Err(Error::Block0(
                Block0Error::InitialMessageNoConsensusLeaderId,
            ));
        }

        let mut ledger = Ledger::empty(settings, static_params);

        let ledger_params = ledger.get_ledger_parameters();

        for content in content_iter {
            match content {
                Message::Initial(_) => {
                    return Err(Error::Block0(Block0Error::InitialMessageMany));
                }
                Message::OldUtxoDeclaration(old) => {
                    ledger.oldutxos = apply_old_declaration(ledger.oldutxos, old)?;
                }
                Message::Transaction(authenticated_tx) => {
                    if authenticated_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0(Block0Error::TransactionHasInput));
                    }
                    if authenticated_tx.witnesses.len() != 0 {
                        return Err(Error::Block0(Block0Error::TransactionHasWitnesses));
                    }
                    let transaction_id = authenticated_tx.transaction.hash();
                    let (new_utxos, new_accounts, new_multisig) =
                        internal_apply_transaction_output(
                            ledger.utxos,
                            ledger.accounts,
                            ledger.multisig,
                            &ledger.static_params,
                            &ledger_params,
                            &transaction_id,
                            &authenticated_tx.transaction.outputs,
                        )?;
                    ledger.utxos = new_utxos;
                    ledger.accounts = new_accounts;
                    ledger.multisig = new_multisig;
                }
                Message::UpdateProposal(_) => {
                    return Err(Error::Block0(Block0Error::HasUpdateProposal));
                }
                Message::UpdateVote(_) => {
                    return Err(Error::Block0(Block0Error::HasUpdateVote));
                }
                Message::Certificate(authenticated_cert_tx) => {
                    if authenticated_cert_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0(Block0Error::TransactionHasInput));
                    }
                    if authenticated_cert_tx.witnesses.len() != 0 {
                        return Err(Error::Block0(Block0Error::TransactionHasWitnesses));
                    }
                    if authenticated_cert_tx.transaction.outputs.len() != 0 {
                        return Err(Error::Block0(Block0Error::TransactionHasOutput));
                    }
                    let (new_delegation, action) = ledger
                        .delegation
                        .apply(&authenticated_cert_tx.transaction.extra)?;
                    ledger.delegation = new_delegation;
                    ledger.apply_delegation_action(action)?;
                }
            }
        }

        ledger.validate_utxo_total_value()?;
        Ok(ledger)
    }

    /// Try to apply messages to a State, and return the new State if succesful
    pub fn apply_block<'a, I>(
        &'a self,
        ledger_params: &LedgerParameters,
        contents: I,
        date: BlockDate,
        chain_length: ChainLength,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Message>,
    {
        let mut new_ledger = self.clone();

        new_ledger.chain_length = self.chain_length.next();

        if chain_length != new_ledger.chain_length {
            return Err(Error::WrongChainLength {
                actual: chain_length,
                expected: new_ledger.chain_length,
            });
        }

        if date <= new_ledger.date {
            return Err(Error::NonMonotonicDate {
                block_date: date,
                chain_date: new_ledger.date,
            });
        }

        let (updates, settings) =
            new_ledger
                .updates
                .process_proposals(new_ledger.settings, new_ledger.date, date)?;
        new_ledger.updates = updates;
        new_ledger.settings = settings;

        for content in contents {
            match content {
                Message::Initial(_) => return Err(Error::Block0(Block0Error::OnlyMessageReceived)),
                Message::OldUtxoDeclaration(_) => {
                    return Err(Error::Block0(Block0Error::OnlyMessageReceived))
                }
                Message::Transaction(authenticated_tx) => {
                    let (new_ledger_, _fee) =
                        new_ledger.apply_transaction(&authenticated_tx, &ledger_params)?;
                    new_ledger = new_ledger_;
                }
                Message::UpdateProposal(update_proposal) => {
                    new_ledger =
                        new_ledger.apply_update_proposal(content.id(), &update_proposal, date)?;
                }
                Message::UpdateVote(vote) => {
                    new_ledger = new_ledger.apply_update_vote(&vote)?;
                }
                Message::Certificate(authenticated_cert_tx) => {
                    let (new_ledger_, _fee) =
                        new_ledger.apply_certificate(authenticated_cert_tx, &ledger_params)?;
                    new_ledger = new_ledger_;
                }
            }
        }

        new_ledger.date = date;

        Ok(new_ledger)
    }

    pub fn apply_transaction<Extra>(
        mut self,
        signed_tx: &AuthenticatedTransaction<Address, Extra>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error>
    where
        Extra: property::Serialize,
        LinearFee: FeeAlgorithm<Transaction<Address, Extra>>,
    {
        let transaction_id = signed_tx.transaction.hash();
        let fee = dyn_params
            .fees
            .calculate(&signed_tx.transaction)
            .map(Ok)
            .unwrap_or(Err(Error::FeeCalculationError(ValueError::Overflow)))?;
        self = internal_apply_transaction(
            self,
            dyn_params,
            &transaction_id,
            &signed_tx.transaction.inputs[..],
            &signed_tx.transaction.outputs[..],
            &signed_tx.witnesses[..],
            fee,
        )?;
        Ok((self, fee))
    }

    pub fn apply_update(mut self, update: &update::UpdateProposal) -> Result<Self, Error> {
        self.settings = self.settings.apply(&update.changes)?;
        Ok(self)
    }

    pub fn apply_update_proposal(
        mut self,
        proposal_id: update::UpdateProposalId,
        proposal: &update::SignedUpdateProposal,
        cur_date: BlockDate,
    ) -> Result<Self, Error> {
        self.updates =
            self.updates
                .apply_proposal(proposal_id, proposal, &self.settings, cur_date)?;
        Ok(self)
    }

    pub fn apply_update_vote(mut self, vote: &update::SignedUpdateVote) -> Result<Self, Error> {
        self.updates = self.updates.apply_vote(vote, &self.settings)?;
        Ok(self)
    }

    pub fn apply_certificate(
        mut self,
        auth_cert: &AuthenticatedTransaction<Address, certificate::Certificate>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error> {
        let verified = auth_cert.transaction.extra.verify();
        if verified == chain_crypto::Verification::Failed {
            return Err(Error::CertificateInvalidSignature);
        };
        let (new_ledger, fee) = self.apply_transaction(auth_cert, dyn_params)?;
        self = new_ledger;
        let (new_delegation, action) = self.delegation.apply(&auth_cert.transaction.extra)?;
        self.delegation = new_delegation;
        self.apply_delegation_action(action)?;
        Ok((self, fee))
    }

    #[inline]
    fn apply_delegation_action(&mut self, actions: CertificateApplyOutput) -> Result<(), Error> {
        match actions {
            CertificateApplyOutput::None => {}
            CertificateApplyOutput::CreateAccount(stake_key_id) => {
                let account = stake_key_id.0.clone().into();
                if !self.accounts.exists(&account) {
                    self.accounts = self.accounts.add_account(&account, Value::zero())?;
                } else {
                    // it is possible the account already exists, in this case
                    // we don't need to do anything
                }
            }
        }
        Ok(())
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        stake::get_distribution(&self.delegation, &self.utxos)
    }

    /// access the ledger static parameters
    pub fn get_static_parameters(&self) -> &LedgerStaticParameters {
        self.static_params.as_ref()
    }

    pub fn accounts(&self) -> &account::Ledger {
        &self.accounts
    }

    pub fn get_ledger_parameters(&self) -> LedgerParameters {
        LedgerParameters {
            fees: *self.settings.linear_fees,
            allow_account_creation: self.settings.allow_account_creation,
        }
    }

    pub fn consensus_version(&self) -> ConsensusVersion {
        self.settings.consensus_version
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.utxos.iter()
    }

    pub fn chain_length(&self) -> ChainLength {
        self.chain_length
    }

    pub fn date(&self) -> BlockDate {
        self.date
    }

    fn validate_utxo_total_value(&self) -> Result<(), Error> {
        let old_utxo_values = self.oldutxos.iter().map(|entry| entry.output.value);
        let new_utxo_values = self.utxos.iter().map(|entry| entry.output.value);
        let account_value = self
            .accounts
            .get_total_value()
            .map_err(|_| Error::Block0(Block0Error::UtxoTotalValueTooBig))?;
        let multisig_value = self
            .multisig
            .get_total_value()
            .map_err(|_| Error::Block0(Block0Error::UtxoTotalValueTooBig))?;
        let all_utxo_values = old_utxo_values
            .chain(new_utxo_values)
            .chain(Some(account_value))
            .chain(Some(multisig_value));
        Value::sum(all_utxo_values)
            .map_err(|_| Error::Block0(Block0Error::UtxoTotalValueTooBig))?;
        Ok(())
    }
}

fn apply_old_declaration(
    mut utxos: utxo::Ledger<legacy::OldAddress>,
    decl: &legacy::UtxoDeclaration,
) -> Result<utxo::Ledger<legacy::OldAddress>, Error> {
    assert!(decl.addrs.len() < 255);
    let txid = decl.hash();
    let mut outputs = Vec::with_capacity(decl.addrs.len());
    for (i, d) in decl.addrs.iter().enumerate() {
        let output = Output {
            address: d.0.clone(),
            value: d.1,
        };
        outputs.push((i as u8, output))
    }
    utxos = utxos.add(&txid, &outputs)?;
    Ok(utxos)
}

/// Apply the transaction
fn internal_apply_transaction(
    mut ledger: Ledger,
    dyn_params: &LedgerParameters,
    transaction_id: &TransactionId,
    inputs: &[Input],
    outputs: &[Output<Address>],
    witnesses: &[Witness],
    fee: Value,
) -> Result<Ledger, Error> {
    assert!(inputs.len() < 255);
    assert!(outputs.len() < 255);
    assert!(witnesses.len() < 255);

    if inputs.len() == 0 {
        return Err(Error::TransactionHasNoInput);
    }

    // 1. verify that number of signatures matches number of
    // transactions
    if inputs.len() != witnesses.len() {
        return Err(Error::NotEnoughSignatures(inputs.len(), witnesses.len()));
    }

    // 2. validate inputs of transaction by gathering what we know of it,
    // then verifying the associated witness
    for (input, witness) in inputs.iter().zip(witnesses.iter()) {
        match input.to_enum() {
            InputEnum::UtxoInput(utxo) => {
                ledger = input_utxo_verify(ledger, transaction_id, &utxo, witness)?
            }
            InputEnum::AccountInput(account_id, value) => {
                let (single, multi) = input_account_verify(
                    ledger.accounts,
                    ledger.multisig,
                    &ledger.static_params.block0_initial_hash,
                    transaction_id,
                    &account_id,
                    value,
                    witness,
                )?;
                ledger.accounts = single;
                ledger.multisig = multi;
            }
        }
    }

    // 3. verify that transaction sum is zero.
    let total_input =
        Value::sum(inputs.iter().map(|i| i.value)).map_err(|e| Error::UtxoInputsTotal(e))?;
    let total_output = Value::sum(inputs.iter().map(|i| i.value).chain(std::iter::once(fee)))
        .map_err(|e| Error::UtxoOutputsTotal(e))?;
    if total_input != total_output {
        return Err(Error::NotBalanced(total_input, total_output));
    }

    // 4. add the new outputs
    let (new_utxos, new_accounts, new_multisig) = internal_apply_transaction_output(
        ledger.utxos,
        ledger.accounts,
        ledger.multisig,
        &ledger.static_params,
        dyn_params,
        transaction_id,
        outputs,
    )?;
    ledger.utxos = new_utxos;
    ledger.accounts = new_accounts;
    ledger.multisig = new_multisig;

    Ok(ledger)
}

fn internal_apply_transaction_output(
    mut utxos: utxo::Ledger<Address>,
    mut accounts: account::Ledger,
    mut multisig: multisig::Ledger,
    static_params: &LedgerStaticParameters,
    dyn_params: &LedgerParameters,
    transaction_id: &TransactionId,
    outputs: &[Output<Address>],
) -> Result<(utxo::Ledger<Address>, account::Ledger, multisig::Ledger), Error> {
    let mut new_utxos = Vec::new();
    for (index, output) in outputs.iter().enumerate() {
        // Reject zero-valued outputs.
        if output.value == Value::zero() {
            return Err(Error::ZeroOutput(output.clone()));
        }

        if output.address.discrimination() != static_params.discrimination {
            return Err(Error::InvalidDiscrimination);
        }
        match output.address.kind() {
            Kind::Single(_) | Kind::Group(_, _) => {
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Account(identifier) => {
                // don't have a way to make a newtype ref from the ref so .clone()
                let account = identifier.clone().into();
                accounts = match accounts.add_value(&account, output.value) {
                    Ok(accounts) => accounts,
                    Err(account::LedgerError::NonExistent) if dyn_params.allow_account_creation => {
                        // if the account was not existent and that we allow creating
                        // account out of the blue, then fallback on adding the account
                        accounts.add_account(&account, output.value)?
                    }
                    Err(error) => return Err(error.into()),
                };
            }
            Kind::Multisig(identifier) => {
                let identifier = multisig::Identifier::from(identifier.clone());
                multisig = multisig.add_value(&identifier, output.value)?;
            }
        }
    }

    utxos = utxos.add(transaction_id, &new_utxos)?;
    Ok((utxos, accounts, multisig))
}

fn input_utxo_verify(
    mut ledger: Ledger,
    transaction_id: &TransactionId,
    utxo: &UtxoPointer,
    witness: &Witness,
) -> Result<Ledger, Error> {
    match witness {
        Witness::Account(_) => Err(Error::ExpectingUtxoWitness),
        Witness::Multisig(_) => Err(Error::ExpectingUtxoWitness),
        Witness::OldUtxo(xpub, signature) => {
            let (old_utxos, associated_output) = ledger
                .oldutxos
                .remove(&utxo.transaction_id, utxo.output_index)?;

            ledger.oldutxos = old_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            };

            if legacy::oldaddress_from_xpub(&associated_output.address, xpub) {
                return Err(Error::OldUtxoInvalidPublicKey(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            let data_to_verify =
                WitnessUtxoData::new(&ledger.static_params.block0_initial_hash, &transaction_id);
            let verified = signature.verify(&xpub, &data_to_verify);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::OldUtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            Ok(ledger)
        }
        Witness::Utxo(signature) => {
            let (new_utxos, associated_output) = ledger
                .utxos
                .remove(&utxo.transaction_id, utxo.output_index)?;
            ledger.utxos = new_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            }

            let data_to_verify =
                WitnessUtxoData::new(&ledger.static_params.block0_initial_hash, &transaction_id);
            let verified = signature.verify(
                &associated_output.address.public_key().unwrap(),
                &data_to_verify,
            );
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::UtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };
            Ok(ledger)
        }
    }
}

fn input_account_verify(
    mut ledger: account::Ledger,
    mut mledger: multisig::Ledger,
    block0_hash: &HeaderHash,
    transaction_id: &TransactionId,
    account: &AccountIdentifier,
    value: Value,
    witness: &Witness,
) -> Result<(account::Ledger, multisig::Ledger), Error> {
    // .remove_value() check if there's enough value and if not, returns a Err.

    match witness {
        Witness::OldUtxo(_, _) => return Err(Error::ExpectingAccountWitness),
        Witness::Utxo(_) => return Err(Error::ExpectingAccountWitness),
        Witness::Account(sig) => {
            // refine account to a single account identifier
            let account = account
                .to_single_account()
                .ok_or(Error::AccountIdentifierInvalid)?;

            let (new_ledger, spending_counter) = ledger.remove_value(&account, value)?;
            ledger = new_ledger;

            let tidsc = WitnessAccountData::new(block0_hash, transaction_id, &spending_counter);
            let verified = sig.verify(&account.clone().into(), &tidsc);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::AccountInvalidSignature(
                    account.clone(),
                    witness.clone(),
                ));
            };
            Ok((ledger, mledger))
        }
        Witness::Multisig(msignature) => {
            // refine account to a multisig account identifier
            let account = account.to_multi_account();

            let (new_ledger, declaration, spending_counter) =
                mledger.remove_value(&account, value)?;

            let data_to_verify =
                WitnessMultisigData::new(&block0_hash, &transaction_id, &spending_counter);
            if msignature.verify(declaration, &data_to_verify) != true {
                return Err(Error::MultisigInvalidSignature(account, witness.clone()));
            }
            mledger = new_ledger;

            Ok((ledger, mledger))
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::key::{SpendingPublicKey, SpendingSecretKey};
    use crate::message::config;
    use crate::milli::Milli;
    use chain_addr::{Address, Discrimination, Kind};
    use chain_crypto::SecretKey;
    use rand::{CryptoRng, RngCore};

    pub fn make_key<R: RngCore + CryptoRng>(
        rng: &mut R,
        discrimination: &Discrimination,
    ) -> (SpendingSecretKey, SpendingPublicKey, Address) {
        let sk = SpendingSecretKey::generate(rng);
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        (sk, pk, user_address)
    }

    macro_rules! assert_err {
        ($left: expr, $right: expr) => {
            match &($left) {
                left_val => match &($right) {
                    Err(e) => {
                        if !(e == left_val) {
                            panic!(
                                "assertion failed: error mismatch \
                                 (left: `{:?}, right: `{:?}`)",
                                *left_val, *e
                            )
                        }
                    }
                    Ok(_) => panic!(
                        "assertion failed: expected error {:?} but got success",
                        *left_val
                    ),
                },
            }
        };
    }

    #[test]
    pub fn utxo() {
        let block0_hash = HeaderHash::hash_bytes(&[1, 2, 3]);
        let discrimination = Discrimination::Test;
        let mut ie = config::ConfigParams::new();
        ie.push(ConfigParam::Discrimination(Discrimination::Test));
        ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));
        let leader_pub_key = SecretKey::generate(rand::thread_rng()).to_public();
        ie.push(ConfigParam::AddBftLeader(leader_pub_key.into()));
        ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
        ie.push(ConfigParam::SlotDuration(10));
        ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            Milli::HALF,
        ));
        ie.push(ConfigParam::SlotsPerEpoch(21600));

        let mut rng = rand::thread_rng();
        let (sk1, _pk1, user1_address) = make_key(&mut rng, &discrimination);
        let (_sk2, _pk2, user2_address) = make_key(&mut rng, &discrimination);
        let value = Value(42000);

        let output0 = Output {
            address: user1_address.clone(),
            value: value,
        };

        let first_trans = AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![output0],
                extra: NoExtra,
            },
            witnesses: vec![],
        };
        let tx0_id = first_trans.transaction.hash();

        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };

        let messages = [Message::Initial(ie), Message::Transaction(first_trans)];
        let ledger = Ledger::new(block0_hash, &messages).unwrap();
        let dyn_params = ledger.get_ledger_parameters();

        {
            let ledger = ledger.clone();
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
                extra: NoExtra,
            };
            let signed_tx = AuthenticatedTransaction {
                transaction: tx,
                witnesses: vec![],
            };
            let r = ledger.apply_transaction(&signed_tx, &dyn_params);
            assert_err!(Error::NotEnoughSignatures(1, 0), r)
        }

        {
            let ledger = ledger.clone();
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
                extra: NoExtra,
            };
            let txid = tx.hash();
            let w1 = Witness::new_utxo(&block0_hash, &txid, &sk1);
            let signed_tx = AuthenticatedTransaction {
                transaction: tx,
                witnesses: vec![w1],
            };
            let r = ledger.apply_transaction(&signed_tx, &dyn_params);
            assert!(r.is_ok())
        }
    }
}
