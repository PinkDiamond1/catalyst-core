use crate::task::ExecTask;
use chrono::{Duration, Utc};
use rand::Rng;
use std::collections::HashSet;
use std::io;
use std::iter::FromIterator;
use structopt::StructOpt;
use vit_servicing_station_lib::{
    db::{
        load_db_connection_pool, models::api_tokens::APITokenData,
        queries::api_tokens::insert_token_data, DBConnection,
    },
    v0::api_token::APIToken,
};

#[derive(Debug, PartialEq, StructOpt)]
pub enum APITokenCmd {
    /// Add provided tokens to database. If --tokens is not provided the binary will read them from the `stdin`
    Add {
        /// List of tokens in URL safe base64. If --tokens is not provided the binary will read them from the `stdin`
        #[structopt(long = "tokens")]
        tokens: Option<Vec<String>>,

        /// URL of the vit-servicing-station database to interact with
        #[structopt(long = "db-url")]
        db_url: String,
    },

    /// Generate API tokens, URL safe base64 encoded.
    Generate {
        /// Number of tokens to generate
        #[structopt(long = "n", default_value = "1")]
        n: usize,

        /// Size of the token
        #[structopt(long = "size", default_value = "10")]
        size: usize,
    },
}

impl APITokenCmd {
    fn generate(n: usize, size: usize) -> Vec<String> {
        (0..n)
            .map(|_| {
                let random_bytes: Vec<u8> =
                    (0..size).map(|_| rand::thread_rng().gen::<u8>()).collect();
                base64::encode_config(random_bytes, base64::URL_SAFE_NO_PAD)
            })
            .collect()
    }

    fn add_tokens_from_stream(db_conn: &DBConnection) -> io::Result<()> {
        let mut base64_tokens: Vec<String> = Vec::new();
        let mut input = String::new();
        while let Ok(n) = io::stdin().read_line(&mut input) {
            if n == 0 {
                break;
            }
            // pop the trailing `\n`
            input.pop();
            base64_tokens.push(input.clone());
        }
        APITokenCmd::add_tokens(&base64_tokens, db_conn)
    }

    fn add_tokens(base64_tokens: &[String], db_conn: &DBConnection) -> io::Result<()> {
        // filter duplicated tokens
        let base64_tokens: HashSet<String> = HashSet::from_iter(base64_tokens.iter().cloned());
        for base64_token in base64_tokens {
            let token =
                base64::decode_config(&base64_token, base64::URL_SAFE_NO_PAD).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "base64 encoded token `{}` is not valid due to:\n {}",
                            base64_token, e
                        ),
                    )
                })?;
            let api_token_data = APITokenData {
                token: APIToken::new(token),
                creation_time: Utc::now().timestamp(),
                expire_time: (Utc::now() + Duration::days(365)).timestamp(),
            };
            insert_token_data(api_token_data, db_conn)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;
        }
        Ok(())
    }

    fn handle_api_token_add(tokens: &Option<Vec<String>>, db_url: &str) -> io::Result<()> {
        // check if db file exists
        if !std::path::Path::new(db_url).exists() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} url does not exists", db_url.to_string()),
            ));
        }

        let pool = load_db_connection_pool(db_url)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))?;
        let db_conn = pool
            .get()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, format!("{}", e)))?;

        match tokens {
            // if not tokens are provided then listen to stdin for input ones
            None => APITokenCmd::add_tokens_from_stream(&db_conn),
            // process the provided tokens
            Some(tokens) => APITokenCmd::add_tokens(tokens, &db_conn),
        }
    }

    fn handle_generate(n: usize, size: usize) -> io::Result<()> {
        let tokens = APITokenCmd::generate(n, size);
        for token in tokens {
            println!("{}", token);
        }
        Ok(())
    }
}

impl ExecTask for APITokenCmd {
    type ResultValue = ();

    fn exec(&self) -> std::io::Result<()> {
        match self {
            APITokenCmd::Add { tokens, db_url } => {
                APITokenCmd::handle_api_token_add(tokens, db_url)
            }
            APITokenCmd::Generate { n, size } => APITokenCmd::handle_generate(*n, *size),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use vit_servicing_station_lib::db::{
        load_db_connection_pool, migrations::initialize_db_with_migration,
        queries::api_tokens::query_token_data_by_token,
    };

    #[test]
    fn generate_token() {
        let size = 10;
        let n = 10;
        let tokens = APITokenCmd::generate(n, size);
        assert_eq!(tokens.len(), n);
        tokens.iter().for_each(|token| {
            assert_eq!(
                base64::decode_config(token, base64::URL_SAFE_NO_PAD)
                    .unwrap()
                    .len(),
                size
            )
        })
    }

    #[test]
    fn add_token() {
        let tokens = APITokenCmd::generate(10, 10);
        let connection_pool = load_db_connection_pool("").unwrap();
        initialize_db_with_migration(&connection_pool);
        let db_conn = connection_pool.get().unwrap();
        APITokenCmd::add_tokens(&tokens, &db_conn).unwrap();
        for token in tokens
            .iter()
            .map(|t| base64::decode_config(t, base64::URL_SAFE_NO_PAD).unwrap())
        {
            assert!(query_token_data_by_token(token.as_ref(), &db_conn)
                .unwrap()
                .is_some());
        }
    }
}
