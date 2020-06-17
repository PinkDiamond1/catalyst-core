use crate::common::{
    jormungandr::JormungandrProcess,
    network::{self, wallet},
};
use std::{cmp::PartialOrd, fmt::Display};

use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::NodeStats;
use jormungandr_testing_utils::testing::FragmentSender;

const PASSIVE: &str = "PASSIVE";
const LEADER_CLIENT: &str = "LEADER_CLIENT";
const LEADER: &str = "LEADER";

#[test]
pub fn passive_node_last_block_info() {
    let mut network_controller = network::builder()
        .single_trust_direction(PASSIVE, LEADER)
        .initials(vec![
            wallet("alice").with(1_000_000).delegated_to(LEADER),
            wallet("bob").with(1_000_000),
        ])
        .build()
        .unwrap();

    let leader = network_controller.spawn_and_wait(LEADER);
    let passive = network_controller.spawn_as_passive_and_wait(PASSIVE);

    let mut alice = network_controller.wallet("alice").unwrap();
    let mut bob = network_controller.wallet("bob").unwrap();

    let stats_before = passive
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        Default::default(),
    );

    fragment_sender
        .send_transactions_round_trip(10, &mut alice, &mut bob, &leader, 100.into())
        .expect("fragment send error");

    assert_last_stats_are_updated(stats_before, &passive);
}

#[test]
pub fn leader_node_last_block_info() {
    let mut network_controller = network::builder()
        .single_trust_direction(LEADER_CLIENT, LEADER)
        .initials(vec![
            wallet("alice").with(1_000_000).delegated_to(LEADER),
            wallet("bob").with(1_000_000),
        ])
        .build()
        .unwrap();

    let leader = network_controller.spawn_and_wait(LEADER);
    let leader_client = network_controller.spawn_and_wait(LEADER_CLIENT);

    let mut alice = network_controller.wallet("alice").unwrap();
    let mut bob = network_controller.wallet("bob").unwrap();

    let stats_before = leader_client
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        Default::default(),
    );

    fragment_sender
        .send_transactions_round_trip(10, &mut alice, &mut bob, &leader, 100.into())
        .expect("fragment send error");

    assert_last_stats_are_updated(stats_before, &leader_client);
}

fn assert_last_stats_are_updated(stats_before: NodeStats, node: &JormungandrProcess) {
    let stats_after = node
        .rest()
        .stats()
        .expect("cannot get stats at end")
        .stats
        .expect("empty stats");

    compare_stats_element(
        stats_before.last_block_content_size,
        stats_after.last_block_content_size,
        "last block content size",
    );

    let before_last_block_date: BlockDate = stats_before.last_block_date.unwrap().parse().unwrap();
    let after_last_block_date: BlockDate = stats_after.last_block_date.unwrap().parse().unwrap();

    compare_stats_element(
        before_last_block_date,
        after_last_block_date,
        "last block date",
    );
    compare_stats_element(
        stats_before.last_block_fees,
        stats_after.last_block_fees,
        "last block fees size",
    );
    compare_stats_element(
        stats_before.last_block_hash.unwrap(),
        stats_after.last_block_hash.unwrap(),
        "last block hash",
    );
    compare_stats_element(
        stats_before.last_block_sum,
        stats_after.last_block_sum,
        "last block sum",
    );
    compare_stats_element(
        stats_before.last_block_time.unwrap(),
        stats_after.last_block_time.unwrap(),
        "last block time",
    );
    compare_stats_element(
        stats_before.last_block_tx,
        stats_after.last_block_tx,
        "last block tx",
    );
}

fn compare_stats_element<T>(before_value: T, after_value: T, info: &str)
where
    T: Display + PartialOrd,
{
    assert!(
        before_value < after_value,
        "{} should to be updated. {} vs {}",
        info,
        before_value,
        after_value,
    );
}
