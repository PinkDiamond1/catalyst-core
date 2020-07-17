use crate::common::{
    data,
    startup::{db::DbBuilder, quick_start, server::ServerBootstrapper},
};
use assert_fs::TempDir;
use reqwest::StatusCode;

#[test]
pub fn get_funds_list_is_not_empty() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap();
    let (server, hash) = quick_start(&temp_dir)?;
    server
        .rest_client_with_token(&hash)
        .funds()
        .expect("cannot get funds");
    Ok(())
}

#[test]
pub fn get_funds_by_id() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let expected_fund = data::funds().first().unwrap().clone();
    let (token, hash) = data::token();

    let db_path = DbBuilder::new()
        .with_token(token)
        .with_funds(vec![expected_fund.clone()])
        .build(&temp_dir)?;

    let server = ServerBootstrapper::new()
        .with_db_path(db_path.to_str().unwrap())
        .start()?;

    let rest_client = server.rest_client_with_token(&hash);

    let actual_fund = rest_client.fund(&expected_fund.id.to_string())?;
    assert_eq!(actual_fund, expected_fund);

    // non existing
    assert_eq!(rest_client.fund_raw("2")?.status(), StatusCode::NOT_FOUND);
    // malformed index
    assert_eq!(rest_client.fund_raw("a")?.status(), StatusCode::NOT_FOUND);
    // overflow index
    assert_eq!(
        rest_client.fund_raw("3147483647")?.status(),
        StatusCode::NOT_FOUND
    );

    Ok(())
}
