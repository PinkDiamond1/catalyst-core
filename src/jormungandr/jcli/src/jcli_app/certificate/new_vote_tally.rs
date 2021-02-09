use crate::jcli_app::certificate::{write_cert, Error};
use crate::jcli_app::utils::vote;
use chain_impl_mockchain::certificate::{
    Certificate, DecryptedPrivateTally, DecryptedPrivateTallyProposal, VotePlanId, VoteTally,
};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::{PrivateTallyState, Tally};
use std::convert::TryInto;
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote tally certificate
///
/// voteplan id needs to be provided
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum VoteTallyRegistration {
    Public(PublicTally),
    Private(PrivateTally),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct PublicTally {
    /// vote plan id
    ///
    /// the vote plan identifier on the blockchain
    #[structopt(long = "vote-plan-id")]
    pub id: VotePlanId,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct PrivateTally {
    /// path to the json file containing the tally shares
    #[structopt(long)]
    pub shares: PathBuf,

    /// path to the json file containing the vote plan result
    #[structopt(long)]
    pub vote_plan: PathBuf,

    /// path to the json file containing the vote plan result
    #[structopt(long)]
    pub vote_plan_id: Option<Hash>,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long)]
    pub output: Option<PathBuf>,
}

impl VoteTallyRegistration {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            VoteTallyRegistration::Public(public) => public.exec(),
            VoteTallyRegistration::Private(private) => private.exec(),
        }
    }
}

impl PublicTally {
    pub fn exec(self) -> Result<(), Error> {
        let vote_tally = VoteTally::new_public(self.id);
        let cert = Certificate::VoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}

impl PrivateTally {
    pub fn exec(self) -> Result<(), Error> {
        let vote_plan =
            vote::get_vote_plan_by_id(Some(self.vote_plan), self.vote_plan_id.as_ref())?;
        let shares: Vec<Vec<chain_vote::TallyDecryptShare>> =
            vote::read_vote_plan_shares_from_file(
                Some(self.shares),
                vote_plan.proposals.len(),
                None,
            )?
            .try_into()?;

        let tallies = vote_plan
            .proposals
            .into_iter()
            .zip(shares)
            .map(|(prop, shares)| match prop.tally {
                Some(Tally::Private {
                    state: PrivateTallyState::Decrypted { result, .. },
                }) => Ok(DecryptedPrivateTallyProposal {
                    decrypt_shares: shares.into_boxed_slice(),
                    tally_result: result.results().into_boxed_slice(),
                }),
                other => Err(Error::PrivateTallyExpected { found: other }),
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let vote_tally =
            VoteTally::new_private(vote_plan.id.into(), DecryptedPrivateTally::new(tallies));
        let cert = Certificate::VoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}
