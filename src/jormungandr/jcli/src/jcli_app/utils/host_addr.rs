use reqwest::Url;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
pub struct HostAddr {
    /// node API address. Must always have `http://` or `https://` prefix.
    /// E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`
    #[structopt(short, long, env = "JORMUNGANDR_RESTAPI_URL")]
    host: Url,
}

impl HostAddr {
    pub fn with_segments(mut self, segments: &[&str]) -> Result<Self, Error> {
        let result = self.host.path_segments_mut().map(|mut host_segments| {
            host_segments.extend(segments);
        });
        match result {
            Ok(_) => Ok(self),
            Err(_) => Err(Error::HostAddrNotBase { addr: self.host }),
        }
    }

    pub fn into_url(self) -> Url {
        self.host
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Host address '{addr}' isn't valid address base")]
    HostAddrNotBase { addr: Url },
}
