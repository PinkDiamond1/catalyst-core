//! Abstractions for the network subsystem of a blockchain node.

#![warn(clippy::all)]

#[macro_use]
extern crate prost_derive;

pub mod client;
pub mod server;

/// Common type definitions generated from protobuf.
pub mod codes {
    include!(concat!(env!("OUT_DIR"), "/iohk.chain.codes.rs"));
}
