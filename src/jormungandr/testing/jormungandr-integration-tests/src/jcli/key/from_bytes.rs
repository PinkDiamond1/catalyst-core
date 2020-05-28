use crate::common::jcli_wrapper;

use assert_fs::prelude::*;
use assert_fs::NamedTempFile;

#[test]
pub fn test_key_from_bytes_ed25519() {
    transform_key_to_bytes_and_back("ed25519");
}

#[test]
pub fn test_key_from_bytes_curve25519_2hashdh() {
    transform_key_to_bytes_and_back("Curve25519_2HashDH");
}

#[test]
pub fn test_key_from_bytes_sumed25519_12() {
    transform_key_to_bytes_and_back("sumed25519_12");
}

#[test]
pub fn test_key_from_bytes_ed25510bip32() {
    transform_key_to_bytes_and_back("Ed25519Bip32");
}

fn transform_key_to_bytes_and_back(key_type: &str) {
    let private_key = jcli_wrapper::assert_key_generate(&key_type);
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    jcli_wrapper::assert_key_to_bytes(&private_key, byte_key_file.path());
    let key_after_transformation =
        jcli_wrapper::assert_key_from_bytes(byte_key_file.path(), &key_type);

    assert_eq!(
        &private_key, &key_after_transformation,
        "orginal and key after transformation are differnt '{}' vs '{}'",
        &private_key, &key_after_transformation
    );
}

#[test]
pub fn test_from_bytes_for_invalid_key() {
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    byte_key_file.write_str(
        "ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    jcli_wrapper::assert_key_from_bytes_fails(
        byte_key_file.path(),
        "ed25519Extended",
        "Odd number of digits",
    );
}

#[test]
pub fn test_from_bytes_for_unknown_key() {
    let byte_key_file = NamedTempFile::new("byte_file").unwrap();
    byte_key_file.write_str(
        "ed25519e_sk1kp80gevhccz8cnst6x97rmlc9n5fls2nmcqcjfn65vdktt0wy9f3zcf76hp7detq9sz8cmhlcyzw5h3ralf98rdwl4wcwcgaaqna3pgz9qgk0").unwrap();
    jcli_wrapper::assert_key_from_bytes_fails(
        byte_key_file.path(),
        "ed25519Exten",
        "Invalid value for '--type <key-type>':",
    );
}
