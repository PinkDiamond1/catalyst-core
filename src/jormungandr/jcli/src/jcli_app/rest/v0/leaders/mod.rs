use crate::jcli_app::rest::{Error, RestArgs};
use crate::jcli_app::utils::{io, OutputFormat};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Leaders {
    /// Get list of leader IDs
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
    /// Register new leader and get its ID
    Post {
        #[structopt(flatten)]
        args: RestArgs,
        /// File containing YAML with leader secret.
        /// It must have the same format as secret YAML passed to Jormungandr as --secret.
        /// If not provided, YAML will be read from stdin.
        #[structopt(short, long)]
        file: Option<PathBuf>,
    },
    /// Delete leader
    Delete {
        #[structopt(flatten)]
        args: RestArgs,
        /// ID of deleted leader
        id: u32,
    },

    /// Leadership log operations
    Logs(GetLogs),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum GetLogs {
    /// Get leadership log
    Get {
        #[structopt(flatten)]
        args: RestArgs,
        #[structopt(flatten)]
        output_format: OutputFormat,
    },
}

impl Leaders {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Leaders::Get {
                args,
                output_format,
            } => get(args, output_format),
            Leaders::Post { args, file } => post(args, file),
            Leaders::Delete { args, id } => delete(args, id),
            Leaders::Logs(GetLogs::Get {
                args,
                output_format,
            }) => get_logs(args, output_format),
        }
    }
}

fn get(args: RestArgs, output_format: OutputFormat) -> Result<(), Error> {
    let response =
        args.request_json_with_args(&["v0", "leaders"], |client, url| client.get(url))?;
    let formatted = output_format.format_json(response)?;
    println!("{}", formatted);
    Ok(())
}

fn post(args: RestArgs, file: Option<PathBuf>) -> Result<(), Error> {
    let input: serde_json::Value = io::read_yaml(&file)?;
    let response = args.request_text_with_args(&["v0", "leaders"], |client, url| {
        client.post(url).json(&input)
    })?;
    println!("{}", response);
    Ok(())
}

fn delete(args: RestArgs, id: u32) -> Result<(), Error> {
    args.request_with_args(&["v0", "leaders", &id.to_string()], |client, url| {
        client.delete(url)
    })?;
    println!("Success");
    Ok(())
}

fn get_logs(args: RestArgs, output_format: OutputFormat) -> Result<(), Error> {
    let response =
        args.request_json_with_args(&["v0", "leaders", "logs"], |client, url| client.get(url))?;
    let formatted = output_format.format_json(response)?;
    println!("{}", formatted);
    Ok(())
}
