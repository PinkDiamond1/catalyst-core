use super::next_id::NextId;
use crate::jcli_app::rest::{Error, RestArgs};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Subcommand {
    /// Get block
    Get {
        #[structopt(flatten)]
        args: RestArgs,
    },
    /// Get block descendant ID
    NextId(NextId),
}

impl Subcommand {
    pub fn exec(self, block_id: String) -> Result<(), Error> {
        match self {
            Subcommand::Get { args } => exec_get(block_id, args),
            Subcommand::NextId(next_id) => next_id.exec(block_id),
        }
    }
}

fn exec_get(block_id: String, args: RestArgs) -> Result<(), Error> {
    let response =
        args.request_bin_with_args(&["v0", "block", &block_id], |client, url| client.get(url))?;
    println!("{}", hex::encode(&response));
    Ok(())
}
