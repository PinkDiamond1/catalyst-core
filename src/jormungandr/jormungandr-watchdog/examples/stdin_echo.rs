use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use jormungandr_watchdog::{
    service, CoreServices, IntercomMsg, Service, ServiceIdentifier, ServiceState, Settings,
    WatchdogBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{stdin, stdout, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    stream::StreamExt as _,
    time::delay_for,
};

use std::time::Duration;
use tracing;
use tracing_subscriber::fmt::Subscriber;

struct StdinReader {
    state: ServiceState<Self>,
}

struct StdoutWriter {
    state: ServiceState<Self>,
}

#[derive(Debug, IntercomMsg)]
struct WriteMsg(String);

#[async_trait]
impl Service for StdinReader {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "stdin";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = service::NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut stdout = self.state.intercom_with::<StdoutWriter>();
        let mut stdin = BufReader::new(stdin()).lines();

        while let Some(msg) = stdin.next().await {
            match msg {
                Err(err) => {
                    tracing::error!(%err);
                    break;
                }
                Ok(line) if line == "quit" => {
                    self.state.watchdog_controller().clone().shutdown().await;
                    break;
                }
                Ok(line) => {
                    tracing::debug!(%line, "read from stdin");
                    if let Err(err) = stdout.send(WriteMsg(line)).await {
                        tracing::error!(%err);
                        break;
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Service for StdoutWriter {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "stdout";

    type State = service::NoState;
    type Settings = service::NoSettings;
    type IntercomMsg = WriteMsg;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let mut stdout = stdout();

        while let Some(WriteMsg(msg)) = self.state.intercom_mut().recv().await {
            if let Err(err) = stdout.write_all(msg.as_bytes()).await {
                tracing::error!(%err);
                break;
            }
            stdout.write_all("\n".as_bytes()).await.unwrap();
            stdout.flush().await.unwrap();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    INFO,
    WARN,
    DEBUG,
    ERROR,
    TRACE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoggerConfig {
    level: LogLevel,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            level: LogLevel::ERROR,
        }
    }
}

fn log_level_into_tracing_level(level: &LogLevel) -> tracing::Level {
    return match level {
        LogLevel::INFO => tracing::Level::INFO,
        LogLevel::WARN => tracing::Level::WARN,
        LogLevel::DEBUG => tracing::Level::DEBUG,
        LogLevel::ERROR => tracing::Level::ERROR,
        LogLevel::TRACE => tracing::Level::TRACE,
    };
}

impl Settings for LoggerConfig {
    fn add_cli_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
        vec![Arg::with_name("Log level")
            .short("ll")
            .long("log_level")
            .takes_value(true)
            .default_value("Warning")
            .value_name("LOG_LEVEL")
            .help("Services log level: []")]
    }

    fn matches_cli_args<'a>(&mut self, matches: &ArgMatches<'a>) {
        if let Some(level) = matches.value_of("cfg") {
            self.level = match level.to_lowercase().as_str() {
                "info" => LogLevel::INFO,
                "warn" => LogLevel::WARN,
                "debug" => LogLevel::DEBUG,
                "error" => LogLevel::ERROR,
                "trace" => LogLevel::TRACE,
                _ => self.level.clone(),
            };
        }
    }
}

struct LoggerService {
    state: ServiceState<Self>,
}

#[async_trait]
impl Service for LoggerService {
    const SERVICE_IDENTIFIER: &'static str = "logger";
    type State = service::NoState;
    type Settings = LoggerConfig;
    type IntercomMsg = service::NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        let subscriber = Subscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting tracing default failed");
        LoggerService { state }
    }

    async fn start(mut self) {
        let this = &mut self;
        while let Some(cfg) = this.state.settings().updated().await {
            let subscriber = Subscriber::builder()
                .with_max_level(log_level_into_tracing_level(&cfg.level))
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting tracing default failed");
            print!("{:?}", cfg);
        }
    }
}

#[derive(CoreServices)]
struct StdEcho {
    stdin: service::ServiceManager<StdinReader>,
    stdout: service::ServiceManager<StdoutWriter>,
    logger: service::ServiceManager<LoggerService>,
}

fn main() {
    // let subscriber = fmt::Subscriber::builder()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .finish();
    //
    // tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let app = App::new("stdin_echo");
    let watchdog = WatchdogBuilder::<StdEcho>::new(app).build();

    let mut controller = watchdog.control();
    watchdog.spawn(async move {
        controller.start("stdout").await.unwrap();
        controller.start("stdin").await.unwrap();
        controller.start("logger").await.unwrap();
    });
    watchdog.wait_finished();
}
