use super::p2p::P2pService;
use crate::{error::Error, subscription::BlockEvent};

use chain_core::property::{Block, HasHeader};

use futures::prelude::*;

use std::{error, fmt};

/// Interface for the blockchain node service responsible for
/// providing access to blocks.
pub trait BlockService: P2pService {
    /// The type of blockchain block served by this service.
    type Block: Block + HasHeader;

    /// The type of asynchronous futures returned by method `handshake`.
    ///
    /// The future resolves to the identifier of the genesis block of the
    /// chain managed by the service node.
    type HandshakeFuture: Future<Item = <Self::Block as Block>::Id, Error = HandshakeError>;

    /// Requests the identifier of the genesis block from the service node.
    ///
    /// The implementation can also perform version information checks to
    /// ascertain that the client use compatible protocol versions.
    ///
    /// This method should be called first after establishing the client
    /// connection.
    fn handshake(&mut self) -> Self::HandshakeFuture;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = <Self::Block as HasHeader>::Header, Error = Error>;

    fn tip(&mut self) -> Self::TipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `pull_blocks_to_tip`.
    type PullBlocksStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `pull_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksToTipFuture: Future<Item = Self::PullBlocksStream, Error = Error>;

    fn pull_blocks_to_tip(
        &mut self,
        from: &[<Self::Block as Block>::Id],
    ) -> Self::PullBlocksToTipFuture;

    /// The type of an asynchronous stream that provides block headers in
    /// response to method `pull_headers`.
    type PullHeadersStream: Stream<Item = <Self::Block as HasHeader>::Header, Error = Error>;

    /// The type of asynchronous futures returned by method `pull_headers`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullHeadersFuture: Future<Item = Self::PullHeadersStream, Error = Error>;

    /// Requests headers of blocks in the blockchain's chronological order,
    /// in the range between the latest of the given starting points, and
    /// the given ending point. If none of the starting points are found
    /// in the chain on the service side, or if the ending point is not found,
    /// the future will fail with a `NotFound` error.
    fn pull_headers(
        &mut self,
        from: &[<Self::Block as Block>::Id],
        to: &<Self::Block as Block>::Id,
    ) -> Self::PullHeadersFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;

    /// Retrieves the identified blocks in an asynchronous stream.
    fn get_blocks(&mut self, ids: &[<Self::Block as Block>::Id]) -> Self::GetBlocksFuture;

    // The type of an asynchronous stream that provides block headers in
    // response to method `get_headers`.
    //type GetHeadersStream: Stream<Item = <Self::Block as Block>::Header, Error = Error>;

    // The type of asynchronous futures returned by method `get_headers`.
    //
    // The future resolves to a stream that will be used by the protocol
    // implementation to produce a server-streamed response.
    //type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;

    /// The type of asynchronous futures returned by method `push_headers`.
    type PushHeadersFuture: Future<Item = (), Error = Error>;

    /// The outbound counterpart of `pull_headers`, called in response to a
    /// `BlockEvent::Missing` solicitation. A valid way to report that
    /// the solicitation does not refer to blocks found in the local blockchain
    /// is to make the `push_headers` call and fail the outbound stream with
    /// a `NotFound` error.
    fn push_headers<S>(&mut self, headers: S) -> Self::PushHeadersFuture
    where
        S: Stream<Item = <Self::Block as HasHeader>::Header, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by method `upload_blocks`.
    type UploadBlocksFuture: Future<Item = (), Error = Error>;

    /// Uploads blocks to the service in response to `BlockEvent::Solicit`.
    ///
    /// The blocks to send are retrieved asynchronously from the passed stream.
    fn upload_blocks<S>(&mut self, blocks: S) -> Self::UploadBlocksFuture
    where
        S: Stream<Item = Self::Block, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by method `block_subscription`.
    ///
    /// The future resolves to a stream of blocks sent by the remote node
    /// and the identifier of the node in the network.
    type BlockSubscriptionFuture: Future<
        Item = (Self::BlockSubscription, Self::NodeId),
        Error = Error,
    >;

    /// The type of an asynchronous stream that provides notifications
    /// of blocks created or accepted by the remote node.
    type BlockSubscription: Stream<Item = BlockEvent<Self::Block>, Error = Error>;

    /// Establishes a bidirectional stream of notifications for blocks
    /// created or accepted by either of the peers.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn block_subscription<S>(&mut self, outbound: S) -> Self::BlockSubscriptionFuture
    where
        S: Stream<Item = <Self::Block as HasHeader>::Header, Error = Error> + Send + 'static;
}

/// An error that the future returned by `BlockService::handshake` can
/// resolve to.
#[derive(Debug)]
pub enum HandshakeError {
    /// The protocol version reported by the server is not supported.
    /// Carries the reported version in a human-readable form.
    UnsupportedVersion(Box<str>),
    /// Error occurred with the protocol request.
    Rpc(Error),
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandshakeError::UnsupportedVersion(v) => {
                write!(f, "unsupported protocol version {}", v)
            }
            HandshakeError::Rpc(e) => write!(f, "{}", e),
        }
    }
}

impl error::Error for HandshakeError {}

impl From<Error> for HandshakeError {
    fn from(src: Error) -> Self {
        HandshakeError::Rpc(src)
    }
}
