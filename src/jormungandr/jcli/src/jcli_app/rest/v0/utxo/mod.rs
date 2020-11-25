use crate::jcli_app::rest::{config::RestArgs, Error};
use crate::jcli_app::utils::OutputFormat;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Utxo {
    /// hex-encoded ID of the transaction fragment
    fragment_id: String,

    /// index of the transaction output
    output_index: u8,

    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Subcommand {
    /// Get UTxO details
    Get {
        #[structopt(flatten)]
        output_format: OutputFormat,

        #[structopt(flatten)]
        args: RestArgs,
    },
}

impl Utxo {
    pub fn exec(self) -> Result<(), Error> {
        let Subcommand::Get {
            args,
            output_format,
        } = self.subcommand;
        let response = args.request_json_with_args(
            &[
                "v0",
                "utxo",
                &self.fragment_id,
                &self.output_index.to_string(),
            ],
            |client, url| client.get(url),
        )?;
        let formatted = output_format.format_json(response)?;
        println!("{}", formatted);
        Ok(())
    }
}
