use crate::jcli_app::utils::output_file::{self, OutputFile};
mod committee;
mod common_reference_string;
mod decrypt_shares;
mod decryption_tally;
mod encrypting_vote_key;

use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("invalid Hexadecimal")]
    Hex(#[from] hex::FromHexError),
    #[error("error while using random source")]
    Rand(#[from] rand::Error),
    #[error("invalid seed length, expected 32 bytes but received {seed_len}")]
    InvalidSeed { seed_len: usize },
    #[error(transparent)]
    InvalidOutput(#[from] output_file::Error),
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid common reference string")]
    InvalidCrs,
    #[error("threshold should be in range (0..{committee_members:?}] and is {threshold:?}")]
    InvalidThreshold {
        threshold: usize,
        committee_members: usize,
    },
    #[error("invalid committee member index")]
    InvalidCommitteMemberIndex,
    #[error("failed to read encrypted tally bytes")]
    EncryptedTallyRead,
    #[error("failed to read decryption key bytes")]
    DecryptionKeyRead,
    #[error("failed to read share bytes")]
    DecryptionShareRead,
    #[error(transparent)]
    FormatError(#[from] crate::jcli_app::utils::output_format::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    // Committee commands
    Committee(committee::Committee),
    /// Build an encryption vote key
    EncryptingVoteKey(encrypting_vote_key::EncryptingVoteKey),
    /// Build an encryption vote key
    CRS(common_reference_string::CRS),
    /// Create decryption share for private voting tally.
    TallyDecryptionShare(decryption_tally::TallyGenerateDecryptionShare),
}

impl Vote {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Vote::Committee(cmd) => cmd.exec(),
            Vote::EncryptingVoteKey(cmd) => cmd.exec(),
            Vote::CRS(cmd) => cmd.exec(),
            Vote::TallyDecryptionShare(cmd) => cmd.exec(),
        }
    }
}

// FIXME: Duplicated with key.rs
#[derive(Debug)]
struct Seed([u8; 32]);
impl std::str::FromStr for Seed {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = hex::decode(s)?;
        if vec.len() != 32 {
            return Err(Error::InvalidSeed {
                seed_len: vec.len(),
            });
        }
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&vec);
        Ok(Seed(bytes))
    }
}
