use crate::common::jormungandr::{ConfigurationBuilder, Starter};
use assert_fs::TempDir;
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    ledger::governance::TreasuryGovernanceAction,
    value::Value,
};
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_testing_utils::testing::VoteCastsGenerator;
use jormungandr_testing_utils::testing::{
    benchmark_consumption, FragmentStatusProvider, ResourcesUsage, VotePlanBuilder,
};
use jormungandr_testing_utils::{
    testing::{node::time::wait_for_epoch, vote_plan_cert, FragmentSender, FragmentSenderSetup},
    wallet::Wallet,
};
use jortestkit::load::{self, Configuration, Monitor};
use rand::rngs::OsRng;

#[test]
pub fn tally_vote_load_test() {
    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 10_000;
    let temp_dir = TempDir::new().unwrap();
    let mut rng = OsRng;

    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(1_000)
        .collect();

    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(10, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(11, 0))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(&committee, &vote_plan).into();
    let mut funds: Vec<InitialUTxO> = vec![committee.to_initial_fund(initial_fund_per_wallet)];

    let mut config_builder = ConfigurationBuilder::new();
    for voter in voters.iter() {
        funds.push(voter.to_initial_fund(initial_fund_per_wallet));

        if funds.len() >= 254 {
            config_builder.with_funds(funds.clone());
            funds.clear();
        }
    }

    let config = config_builder
        .with_committees(&[&committee.clone()])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let configuration = Configuration::requests_per_thread(5, 5, 100, Monitor::Standard(100), 100);

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::no_verify(),
    );

    let mut benchmark_consumption_monitor =
        benchmark_consumption("tallying_public_vote_with_10_000_votes")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process("Node", jormungandr.pid() as usize)
            .start_async(std::time::Duration::from_secs(30));

    let mut votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );

    let rewards_before = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    wait_for_epoch(5, jormungandr.explorer().clone());

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(6, jormungandr.explorer().clone());

    benchmark_consumption_monitor.stop();

    jormungandr.assert_no_errors_in_log();
}
