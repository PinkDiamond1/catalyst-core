use crate::{generate_settings, generate_wallet};
use wallet_js::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn vote_cast_public_test() {
    let mut wallet = generate_wallet().unwrap();

    let settings = generate_settings().unwrap();

    let vote_plan = VotePlanId::from_bytes(&[
        0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ])
    .unwrap();

    let vote_cast = VoteCast::new(vote_plan, 8, Payload::new_public(0));

    let fragment = wallet
        .sign_transaction(
            &settings,
            BlockDate::new(0, 1),
            0,
            Certificate::vote_cast(vote_cast),
        )
        .unwrap();

    wallet.confirm_transaction(&fragment.id());
}

#[wasm_bindgen_test]
fn vote_cast_private_test() {
    let mut wallet = generate_wallet().unwrap();

    let settings = generate_settings().unwrap();

    let vote_plan = VotePlanId::from_bytes(&[
        0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ])
    .unwrap();

    let vote_cast = VoteCast::new(
        vote_plan.clone(),
        8,
        Payload::new_private(
            vote_plan,
            4,
            0,
            &[
                190, 216, 136, 135, 171, 224, 168, 79, 100, 105, 31, 224, 189, 250, 61, 175, 26,
                108, 214, 151, 161, 63, 7, 174, 7, 88, 137, 16, 206, 57, 201, 39,
            ],
        )
        .unwrap(),
    );

    let fragment = wallet
        .sign_transaction(
            &settings,
            BlockDate::new(0, 1),
            0,
            Certificate::vote_cast(vote_cast),
        )
        .unwrap();

    wallet.confirm_transaction(&fragment.id());
}
