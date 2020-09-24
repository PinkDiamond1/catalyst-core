use crate::common::startup::start_stake_pool;
use crate::common::{
    jcli_wrapper,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{Proposal, Proposals, PushProposal, VoteAction, VotePlan},
    ledger::governance::TreasuryGovernanceAction,
    testing::VoteTestGen,
    value::Value,
    vote::{Choice, CommitteeId, Options, PayloadType},
};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{CommitteeIdDef, Tally, VotePlanStatus},
};
use jormungandr_testing_utils::{
    testing::{vote_plan_cert, FragmentSender, FragmentSenderSetup},
    wallet::Wallet,
};
use rand::rngs::OsRng;
use rand_core::{CryptoRng, RngCore};

const TEST_COMMITTEE_SIZE: usize = 3;

fn generate_wallets_and_committee<RNG>(rng: &mut RNG) -> (Vec<Wallet>, Vec<CommitteeIdDef>)
where
    RNG: CryptoRng + RngCore,
{
    let mut ids = Vec::new();
    let mut wallets = Vec::new();
    for _i in 0..TEST_COMMITTEE_SIZE {
        let wallet = Wallet::new_account(rng);
        ids.push(wallet.to_committee_id());
        wallets.push(wallet);
    }
    (wallets, ids)
}

#[test]
pub fn test_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let (_, mut expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let leader_key_pair = KeyPair::generate(&mut rng);

    let config = ConfigurationBuilder::new()
        .with_leader_key_pair(leader_key_pair.clone())
        .with_committee_ids(expected_committee_ids.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    expected_committee_ids.insert(
        0,
        CommitteeIdDef::from(CommitteeId::from(
            leader_key_pair.identifier().into_public_key(),
        )),
    );

    let actual_committee_ids =
        jcli_wrapper::assert_get_active_voting_committees(&jormungandr.rest_uri());

    assert_eq!(expected_committee_ids, actual_committee_ids);
}

#[test]
pub fn test_get_initial_vote_plan() {
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let (wallets, expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let expected_vote_plan = VoteTestGen::vote_plan();

    let vote_plan_cert = vote_plan_cert(&wallets[0], &expected_vote_plan).into();

    let config = ConfigurationBuilder::new()
        .with_committee_ids(expected_committee_ids.clone())
        .with_certs(vec![vote_plan_cert])
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    assert!(vote_plans.len() == 1);

    let vote_plan = vote_plans.get(0).unwrap();
    assert_eq!(
        vote_plan.id.to_string(),
        expected_vote_plan.to_id().to_string()
    );
}

use chain_core::property::BlockDate as _;

fn proposal_with_3_options() -> Proposal {
    let action = VoteAction::Treasury {
        action: TreasuryGovernanceAction::TransferToRewards { value: Value(10) },
    };

    Proposal::new(
        VoteTestGen::external_proposal_id(),
        Options::new_length(3).unwrap(),
        action.clone(),
    )
}

fn proposals() -> Proposals {
    let mut proposals = Proposals::new();
    for _ in 0..3 {
        assert_eq!(
            PushProposal::Success,
            proposals.push(proposal_with_3_options()),
            "generate_proposal method is only for correct data preparation"
        );
    }
    proposals
}

fn vote_plan_with_3_proposals() -> VotePlan {
    VotePlan::new(
        BlockDate::from_epoch_slot_id(1, 0),
        BlockDate::from_epoch_slot_id(2, 0),
        BlockDate::from_epoch_slot_id(3, 0),
        proposals(),
        PayloadType::Public,
    )
}

#[test]
pub fn test_vote_flow_bft() {
    let favorable_choice = Choice::new(0);
    let initial_fund_per_wallet = 1_000_000;
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let mut alice = Wallet::new_account(&mut rng);
    let mut bob = Wallet::new_account(&mut rng);
    let mut clarice = Wallet::new_account(&mut rng);

    let vote_plan = vote_plan_with_3_proposals();
    let vote_plan_cert = vote_plan_cert(&alice, &vote_plan).into();
    let wallets = [&alice, &bob, &clarice];
    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.into_initial_fund(initial_fund_per_wallet))
                .collect(),
        )
        .with_committees(&wallets)
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_slot_duration(1)
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let epoch_duration = config.epoch_duration();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::resend_3_times(),
    );

    std::thread::sleep(epoch_duration);
    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut bob, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();

    std::thread::sleep(epoch_duration);
    transaction_sender
        .send_vote_tally(&mut clarice, &vote_plan, &jormungandr)
        .unwrap();

    std::thread::sleep(epoch_duration);

    assert_first_proposal_has_votes(
        2 * initial_fund_per_wallet,
        jormungandr.rest().vote_plan_statuses().unwrap(),
    );
}

fn assert_first_proposal_has_votes(stake: u64, vote_plan_statuses: Vec<VotePlanStatus>) {
    println!("{:?}", vote_plan_statuses);
    let proposal = vote_plan_statuses
        .first()
        .unwrap()
        .proposals
        .first()
        .unwrap();
    assert!(proposal.tally.is_some());
    match proposal.tally.as_ref().unwrap() {
        Tally::Public { result } => {
            let results = result.results();
            assert_eq!(*results.get(0).unwrap(), stake);
            assert_eq!(*results.get(1).unwrap(), 0);
            assert_eq!(*results.get(2).unwrap(), 0);
        }
    }
}

#[ignore]
#[test]
pub fn test_vote_flow_praos() {
    let favorable_choice = Choice::new(0);
    let initial_fund_per_wallet = 1_000_000;
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let mut alice = Wallet::new_account(&mut rng);
    let mut bob = Wallet::new_account(&mut rng);
    let mut clarice = Wallet::new_account(&mut rng);

    let vote_plan = vote_plan_with_3_proposals();

    let vote_plan_cert = vote_plan_cert(&alice, &vote_plan).into();
    let wallets = [&alice, &bob, &clarice];
    let mut config = ConfigurationBuilder::new();
    config
        .with_funds(
            wallets
                .iter()
                .map(|x| x.into_initial_fund(1_000_000))
                .collect(),
        )
        .with_committees(&wallets)
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_slot_duration(1);

    let epoch_duration = config.clone().build(&temp_dir).epoch_duration();
    let (jormungandr, stake_pools) = start_stake_pool(
        &[alice.clone(), bob.clone(), clarice.clone()],
        &[],
        &mut config,
    )
    .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::resend_3_times(),
    );

    std::thread::sleep(epoch_duration);
    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut bob, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();

    std::thread::sleep(epoch_duration);
    transaction_sender
        .send_vote_tally(&mut clarice, &vote_plan, &jormungandr)
        .unwrap();

    std::thread::sleep(epoch_duration);

    assert_first_proposal_has_votes(
        2 * initial_fund_per_wallet,
        jormungandr.rest().vote_plan_statuses().unwrap(),
    );
}
