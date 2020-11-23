use super::next_id::NextId;
use crate::jcli_app::rest::Error;
use crate::jcli_app::utils::{DebugFlag, HostAddr, RestApiSender, TlsCert};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Subcommand {
    /// Get block
    Get {
        #[structopt(flatten)]
        addr: HostAddr,
        #[structopt(flatten)]
        debug: DebugFlag,
        #[structopt(flatten)]
        tls: TlsCert,
    },
    /// Get block descendant ID
    NextId(NextId),
}

impl Subcommand {
    pub fn exec(self, block_id: String) -> Result<(), Error> {
        match self {
            Subcommand::Get { addr, debug, tls } => exec_get(block_id, addr, debug, tls),
            Subcommand::NextId(next_id) => next_id.exec(block_id),
        }
    }
}

fn exec_get(block_id: String, addr: HostAddr, debug: DebugFlag, tls: TlsCert) -> Result<(), Error> {
    let url = addr.with_segments(&["v0", "block", &block_id])?.into_url();
    let builder = reqwest::blocking::Client::new().get(url);
    let response = RestApiSender::new(builder, &debug, &tls).send()?;
    response.ok_response()?;
    let body = response.body().binary();
    println!("{}", hex::encode(&body));
    Ok(())
}
