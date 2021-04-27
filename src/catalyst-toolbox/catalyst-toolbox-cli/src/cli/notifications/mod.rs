pub mod requests;
pub mod responses;
mod send;

use structopt::StructOpt;
use thiserror::Error;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CreateMessageError(#[from] requests::create_message::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum PushNotifications {
    Send(send::SendNotification),
}

impl PushNotifications {
    pub fn exec(self) -> Result<(), Error> {
        use self::PushNotifications::*;
        match self {
            Send(_) => {}
        };
        Ok(())
    }
}
