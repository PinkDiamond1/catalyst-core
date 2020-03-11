//! Representation of the block in the mockchain.
use crate::fragment::{Fragment, FragmentRaw};
use chain_core::mempack::{read_from_raw, ReadBuf, ReadError, Readable};
use chain_core::property;

use std::slice;

mod builder;
mod header;
mod headerraw;
mod leaderlog;

#[cfg(any(test, feature = "property-test-api"))]
pub mod test;

//pub use self::builder::BlockBuilder;
pub use crate::fragment::{BlockContentHash, BlockContentSize, Contents, ContentsBuilder};

pub use self::headerraw::HeaderRaw;
pub use self::leaderlog::LeadersParticipationRecord;
pub use crate::header::{
    BftProof, BftSignature, Common, GenesisPraosProof, Header, HeaderContentEvalContext, HeaderId,
    KESSignature, Proof,
};

pub use builder::builder;

pub use crate::header::{BlockVersion, ChainLength};

pub use crate::date::{BlockDate, BlockDateParseError, Epoch, SlotId};

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub contents: Contents,
}

impl PartialEq for Block {
    fn eq(&self, rhs: &Self) -> bool {
        self.header.hash() == rhs.header.hash()
    }
}
impl Eq for Block {}

impl Block {
    pub fn is_consistent(&self) -> bool {
        let (content_hash, content_size) = self.contents.compute_hash_size();

        &content_hash == &self.header.block_content_hash()
            && content_size == self.header.block_content_size()
    }

    pub fn fragments<'a>(&'a self) -> impl Iterator<Item = &'a Fragment> {
        self.contents.iter()
    }
}

impl property::Block for Block {
    type Id = HeaderId;
    type Date = BlockDate;
    type Version = BlockVersion;
    type ChainLength = ChainLength;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        self.header.hash()
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        self.header.block_parent_hash()
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        self.header.block_date()
    }

    fn version(&self) -> Self::Version {
        self.header.block_version()
    }

    fn chain_length(&self) -> Self::ChainLength {
        self.header.chain_length()
    }
}

impl property::Serialize for Block {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        let header_raw = {
            let mut v = Vec::new();
            self.header.serialize(&mut v)?;
            HeaderRaw(v)
        };
        header_raw.serialize(&mut writer)?;

        for message in self.contents.iter() {
            let message_raw = message.to_raw();
            message_raw.serialize(&mut writer)?;
        }
        Ok(())
    }
}

impl property::Deserialize for Block {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let header_raw = HeaderRaw::deserialize(&mut reader)?;
        let header = read_from_raw::<Header>(header_raw.as_ref())?;

        let mut serialized_content_size = header.block_content_size();
        let mut contents = ContentsBuilder::new();

        while serialized_content_size > 0 {
            let message_raw = FragmentRaw::deserialize(&mut reader)?;
            let message_size = message_raw.size_bytes_plus_size();

            // return error here if message serialize sized is bigger than remaining size

            let message = Fragment::from_raw(&message_raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            contents.push(message);

            serialized_content_size -= message_size as u32;
        }

        Ok(Block {
            header: header,
            contents: contents.into(),
        })
    }
}

impl Readable for Block {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let header_size = buf.get_u16()? as usize;
        let mut header_buf = buf.split_to(header_size)?;
        let header = Header::read(&mut header_buf)?;

        let mut remaining_content_size = header.block_content_size();
        let mut contents = ContentsBuilder::new();

        while remaining_content_size > 0 {
            let message_size = buf.get_u16()?;
            let mut message_buf = buf.split_to(message_size as usize)?;

            // return error here if message serialize sized is bigger than remaining size

            let message = Fragment::read(&mut message_buf)?;
            contents.push(message);

            remaining_content_size -= 2 + message_size as u32;
        }

        Ok(Block {
            header: header,
            contents: contents.into(),
        })
    }
}

impl<'a> property::HasFragments<'a> for &'a Block {
    type Fragment = Fragment;
    type Fragments = slice::Iter<'a, Fragment>;
    fn fragments(self) -> Self::Fragments {
        self.contents.iter_slice()
    }
}

impl property::HasHeader for Block {
    type Header = Header;
    fn header(&self) -> Self::Header {
        self.header.clone()
    }
}

use chain_core::property::Deserialize;
use chain_ser::deser::Serialize;
use chain_ser::packer::Codec;
use std::io::Error;
use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(
    Debug, Clone, Copy, Display, EnumString, IntoStaticStr, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum ConsensusVersion {
    #[strum(to_string = "bft")]
    Bft = 1,
    #[strum(to_string = "genesis")]
    GenesisPraos = 2,
}

impl ConsensusVersion {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(ConsensusVersion::Bft),
            2 => Some(ConsensusVersion::GenesisPraos),
            _ => None,
        }
    }
    pub fn supported_block_versions(self) -> &'static [BlockVersion] {
        match self {
            ConsensusVersion::Bft => &[BlockVersion::Ed25519Signed],
            ConsensusVersion::GenesisPraos => &[BlockVersion::KesVrfproof],
        }
    }

    pub fn from_block_version(block_version: BlockVersion) -> Option<ConsensusVersion> {
        match block_version {
            BlockVersion::Genesis => None,
            BlockVersion::Ed25519Signed => Some(ConsensusVersion::Bft),
            BlockVersion::KesVrfproof => Some(ConsensusVersion::GenesisPraos),
        }
    }
}

impl property::Serialize for ConsensusVersion {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut codec = Codec::new(writer);
        match self {
            ConsensusVersion::Bft => {
                codec.put_u8(1)?;
            }
            ConsensusVersion::GenesisPraos => {
                codec.put_u8(2)?;
            }
        }
        Ok(())
    }
}

impl property::Deserialize for ConsensusVersion {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut codec = Codec::new(reader);
        match codec.get_u8()? {
            1 => Ok(ConsensusVersion::Bft),
            2 => Ok(ConsensusVersion::GenesisPraos),
            code => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unrecognized code {} for ConsensusVersion", code),
            )),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use test::Bencher;

    /*
    #[bench]
    pub fn serialization(&b: &mut Bencher) -> Self {
        let bc = BlockContent::new();

        b.iter(|| {

        })
    }
    */
}
