use crate::jcli_app::certificate::{write_cert, Error};
use crate::jcli_app::utils::io::open_file_read;
use chain_impl_mockchain::certificate::{Certificate, TallyDecryptShares, VotePlanId, VoteTally};
use jormungandr_lib::interfaces::TallyDecryptShare;
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
    /// vote plan id
    ///
    /// the vote plan identifier on the blockchain
    #[structopt(long = "vote-plan-id")]
    pub id: VotePlanId,

    pub encrypted_shares_file: PathBuf,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
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
        let shares = read_shares(self.encrypted_shares_file)?;
        let vote_tally = VoteTally::new_private(self.id, shares);
        let cert = Certificate::VoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}

fn read_shares(file_path: PathBuf) -> Result<TallyDecryptShares, Error> {
    let buff = open_file_read(&Some(file_path))?;
    let serde_shares: Vec<Vec<TallyDecryptShare>> = serde_json::from_reader(buff)?;
    let mut shares = Vec::with_capacity(serde_shares.len());
    for proposal_shares in serde_shares {
        let mut new_shares = Vec::with_capacity(proposal_shares.len());
        for share in proposal_shares {
            new_shares.push(share.try_into()?);
        }
        shares.push(new_shares);
    }
    Ok(TallyDecryptShares::new(shares))
}
