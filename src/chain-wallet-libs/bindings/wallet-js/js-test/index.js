// import * as wasm_wallet from "wallet-js";
const wasm_wallet = import("wallet-js");

const MAX_LANES = 8;

const private_key = [
  200, 101, 150, 194, 209, 32, 136, 133, 219, 31, 227, 101, 132, 6, 170, 15,
  124, 199, 184, 225, 60, 54, 47, 228, 106, 109, 178, 119, 252, 80, 100, 88, 62,
  72, 117, 136, 201, 138, 108, 54, 226, 231, 68, 92, 10, 221, 54, 248, 63, 23,
  28, 181, 204, 253, 129, 85, 9, 209, 156, 211, 142, 203, 10, 243,
];

const block0 = [
  0, 0, 0, 0, 4, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 251, 146, 249, 54, 155,
  37, 136, 184, 153, 203, 182, 12, 114, 167, 205, 91, 102, 99, 2, 218, 205, 64,
  246, 250, 21, 159, 228, 207, 70, 145, 13, 117, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  172, 0, 0, 0, 15, 4, 8, 0, 0, 0, 0, 94, 146, 44, 112, 2, 1, 1, 6, 2, 0, 1, 28,
  24, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 100,
  8, 4, 0, 0, 0, 180, 10, 1, 20, 32, 4, 0, 0, 168, 192, 16, 8, 0, 0, 0, 0, 0, 0,
  0, 100, 18, 4, 0, 1, 144, 0, 12, 4, 0, 0, 0, 100, 30, 4, 0, 0, 0, 100, 44, 1,
  1, 38, 8, 0, 0, 90, 243, 16, 122, 64, 0, 40, 33, 2, 0, 0, 0, 0, 0, 0, 0, 100,
  0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 19, 0, 0, 0, 1, 0, 0, 0, 3, 22,
  32, 2, 88, 224, 101, 87, 239, 165, 12, 43, 148, 165, 133, 196, 159, 69, 171,
  246, 122, 222, 148, 23, 78, 110, 166, 66, 109, 18, 106, 179, 97, 118, 166, 0,
  0, 0, 53, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 5, 166, 163, 192, 68, 122, 235,
  156, 197, 76, 246, 66, 43, 163, 43, 41, 78, 94, 28, 62, 246, 215, 130, 242,
  172, 255, 74, 112, 105, 76, 77, 22, 99, 0, 0, 0, 0, 0, 0, 39, 16, 0, 0, 0, 58,
  0, 13, 4, 0, 0, 0, 0, 0, 166, 163, 192, 68, 122, 235, 156, 197, 76, 246, 66,
  43, 163, 43, 41, 78, 94, 28, 62, 246, 215, 130, 242, 172, 255, 74, 112, 105,
  76, 77, 22, 99, 0, 0, 0, 0, 0, 15, 66, 64, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
  2, 234, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0,
  0, 0, 0, 1, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
  0, 0, 0, 2, 88, 224, 101, 87, 239, 165, 12, 43, 148, 165, 133, 196, 159, 69,
  171, 246, 122, 222, 148, 23, 78, 110, 166, 66, 109, 18, 106, 179, 97, 118,
  166, 132, 194, 26, 115, 221, 213, 185, 216, 77, 157, 74, 154, 38, 166, 198,
  241, 88, 16, 15, 246, 52, 244, 133, 14, 183, 145, 70, 76, 98, 45, 30, 68, 185,
  232, 224, 169, 91, 100, 100, 84, 20, 147, 34, 118, 39, 176, 26, 229, 134, 64,
  248, 62, 218, 117, 95, 225, 133, 137, 253, 55, 26, 145, 16, 11,
];

function generate_wallet() {
  let wallet = wasm_wallet.Wallet.import_keys(private_key);

  let spending_counters = wasm_wallet.SpendingCounters.new();
  for (let lane = 0; lane < MAX_LANES; lane++) {
    let spending_counter = wasm_wallet.SpendingCounter.new(lane, 1);
    spending_counters.add(spending_counter);
  }

  wallet.set_state(BigInt(1000), spending_counters);

  return wallet;
}

function generate_settings() {
  let settings = wasm_wallet.Settings.new(block0);
  return settings;
}

function vote_cast_public_test() {
  let wallet = generate_wallet();
  let settings = generate_settings();

  let vote_plan = wasm_wallet.VotePlanId.from_bytes([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
  ]);
  let payload = wasm_wallet.Payload.new_public(0);
  let vote_cast = wasm_wallet.VoteCast.new(vote_plan, 8, payload);

  let block_date = wasm_wallet.BlockDate.new(0, 1);
  let certificate = wasm_wallet.Certificate.vote_cast(vote_cast);
  let fragment = wallet.sign_transaction(settings, block_date, 0, certificate);

  wallet.confirm_transaction(fragment.id());
}

function vote_cast_private_test() {
  let wallet = generate_wallet();
  let settings = generate_settings();

  let vote_plan = wasm_wallet.VotePlanId.from_bytes([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
  ]);
  let payload = wasm_wallet.Payload.new_private(
    vote_plan,
    4,
    0,
    [
      190, 216, 136, 135, 171, 224, 168, 79, 100, 105, 31, 224, 189, 250, 61,
      175, 26, 108, 214, 151, 161, 63, 7, 174, 7, 88, 137, 16, 206, 57, 201, 39,
    ]
  );
  
  vote_plan = wasm_wallet.VotePlanId.from_bytes([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
  ]);
  let vote_cast = wasm_wallet.VoteCast.new(vote_plan, 8, payload);

  let block_date = wasm_wallet.BlockDate.new(0, 1);
  let certificate = wasm_wallet.Certificate.vote_cast(vote_cast);
  let fragment = wallet.sign_transaction(settings, block_date, 0, certificate);

  wallet.confirm_transaction(fragment.id());
}

vote_cast_public_test();
vote_cast_private_test();