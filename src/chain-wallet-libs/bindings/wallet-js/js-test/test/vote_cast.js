var assert = require("assert");

const MAX_LANES = 8;

const private_key = Buffer.from(
  "c86596c2d1208885db1fe3658406aa0f7cc7b8e13c362fe46a6db277fc5064583e487588c98a6c36e2e7445c0add36f83f171cb5ccfd815509d19cd38ecb0af3",
  "hex"
);

const block0 = Buffer.from(
  "000000000415000000000000000000000000fb92f9369b2588b899cbb60c72a7cd5b666302dacd40f6fa159fe4cf46910d750000000000000000000000000000000000000000000000000000000000000000000000ac0000000f0408000000005e922c70020101060200011c18000000000000000a000000000000000200000000000000640804000000b40a011420040000a8c0100800000000000000641204000190000c04000000641e04000000642c0101260800005af3107a40002821020000000000000064000000000000000d0000000000000013000000010000000316200258e06557efa50c2b94a585c49f45abf67ade94174e6ea6426d126ab36176a60000003500020000000000000000000105a6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d166300000000000027100000003a000d040000000000a6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d166300000000000f424000000001000000000000000002ea000a0000000000000000000000010000000000000002000000000111000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000400000000000000010000000000000258e06557efa50c2b94a585c49f45abf67ade94174e6ea6426d126ab36176a684c21a73ddd5b9d84d9d4a9a26a6c6f158100ff634f4850eb791464c622d1e44b9e8e0a95b6464541493227627b01ae58640f83eda755fe18589fd371a91100b",
  "hex"
);

function generate_wallet(wasm_wallet) {
  let wallet = wasm_wallet.Wallet.import_keys(private_key);

  let spending_counters = wasm_wallet.SpendingCounters.new();
  for (let lane = 0; lane < MAX_LANES; lane++) {
    let spending_counter = wasm_wallet.SpendingCounter.new(lane, 1);
    spending_counters.add(spending_counter);
  }

  wallet.set_state(BigInt(1000), spending_counters);

  assert(wallet.total_value() === BigInt(1000));

  return wallet;
}

function generate_settings(wasm_wallet) {
  let settings = wasm_wallet.Settings.new(block0);
  return settings;
}

describe("vote cast certificate tests", function () {
  it("public", async function () {
    const wasm_wallet = await import("wallet-js");

    let wallet = generate_wallet(wasm_wallet);
    let settings = generate_settings(wasm_wallet);
    let vote_plan = wasm_wallet.VotePlanId.from_bytes(
      Buffer.from(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "hex"
      )
    );
    let payload = wasm_wallet.Payload.new_public(0);
    let vote_cast = wasm_wallet.VoteCast.new(vote_plan, 8, payload);

    let block_date = wasm_wallet.BlockDate.new(0, 1);
    let certificate = wasm_wallet.Certificate.vote_cast(vote_cast);
    let fragment = wallet.sign_transaction(
      settings,
      block_date,
      0,
      certificate
    );

    wallet.confirm_transaction(fragment.id());
  });

  it("private", async function () {
    const wasm_wallet = await import("wallet-js");

    let wallet = generate_wallet(wasm_wallet);
    let settings = generate_settings(wasm_wallet);

    let vote_plan = wasm_wallet.VotePlanId.from_bytes(
      Buffer.from(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "hex"
      )
    );
    let payload = wasm_wallet.Payload.new_private(
      vote_plan,
      4,
      0,
      Buffer.from(
        "bed88887abe0a84f64691fe0bdfa3daf1a6cd697a13f07ae07588910ce39c927",
        "hex"
      )
    );

    vote_plan = wasm_wallet.VotePlanId.from_bytes(
      Buffer.from(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "hex"
      )
    );
    let vote_cast = wasm_wallet.VoteCast.new(vote_plan, 8, payload);

    let block_date = wasm_wallet.BlockDate.new(0, 1);
    let certificate = wasm_wallet.Certificate.vote_cast(vote_cast);
    let fragment = wallet.sign_transaction(
      settings,
      block_date,
      0,
      certificate
    );

    wallet.confirm_transaction(fragment.id());
  });
});
