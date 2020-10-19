#[macro_use(error_chain, bail)]
extern crate error_chain;

mod legacy;
mod node;
mod programs;
#[macro_use]
mod scenario;
mod example_scenarios;
mod interactive;
mod slog;
mod test;

use node::{Node, NodeBlock0, NodeController};
use programs::prepare_command;
use scenario::{
    parse_progress_bar_mode_from_str,
    repository::{parse_tag_from_str, ScenarioResult, ScenariosRepository, Tag},
    Context, ProgressBarMode, Seed,
};

pub use jortestkit::console::style;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
struct CommandArgs {
    /// path or name of the jormungandr node to test
    #[structopt(long = "jormungandr", default_value = "jormungandr")]
    jormungandr: PathBuf,

    /// path or name of the jcli to test
    #[structopt(long = "jcli", default_value = "jcli")]
    jcli: PathBuf,

    /// scenario name
    #[structopt(long = "scenario", default_value = "*")]
    scenario: String,

    /// in order to group scenarios (for example long_running, short running)
    /// one can use tag parameter to run entire set of scenarios
    /// by default all scenarios are run
    #[structopt(
        long = "tag",
        default_value = "All",
        parse(try_from_str = parse_tag_from_str)
    )]
    tag: Tag,

    /// set a directory in which the tests will be run, allowing every details
    /// to be save persistently. By default it will create temporary directories
    /// and will delete the files and documents
    #[structopt(long = "root-dir")]
    testing_directory: Option<PathBuf>,

    /// document the different scenario, creating markdown and dot (graphviz) files
    /// describing the tests initial setup
    ///
    /// The files are created within the `--root-dir`  directory.
    #[structopt(long = "document")]
    generate_documentation: bool,

    /// in some circumstances progress bar can spoil test logs (e.g. on test build job)
    /// if this parametrer value is true, then no progress bar is visible,
    /// but simple log on console enabled
    ///
    /// no progress bar, only simple console output
    #[structopt(
        long = "progress-bar-mode",
        default_value = "Monitor",
        parse(from_str = parse_progress_bar_mode_from_str)
    )]
    progress_bar_mode: ProgressBarMode,

    /// set exit code based on test result
    #[structopt(long = "set-exit-code")]
    set_exit_code: bool,

    /// to set if to reproduce an existing test
    #[structopt(long = "seed")]
    seed: Option<Seed>,

    /// level for all nodes
    #[structopt(long = "log-level", default_value = "info")]
    log_level: String,

    /// report statuses for all unstable tests
    #[structopt(long = "report-unstable")]
    report_unstable: bool,

    /// does not silence panics in tests
    #[structopt(long = "print_panics")]
    print_panics: bool,
}

fn main() {
    let command_args = CommandArgs::from_args();

    std::env::set_var("RUST_BACKTRACE", "full");

    let jormungandr = prepare_command(&command_args.jormungandr);
    let jcli = prepare_command(&command_args.jcli);
    let progress_bar_mode = command_args.progress_bar_mode;
    let seed = command_args
        .seed
        .unwrap_or_else(|| Seed::generate(rand::rngs::OsRng));
    let testing_directory = command_args.testing_directory;
    let generate_documentation = command_args.generate_documentation;
    let log_level = command_args.log_level;

    let context = Context::new(
        seed,
        jormungandr,
        jcli,
        testing_directory,
        generate_documentation,
        progress_bar_mode,
        log_level,
    );

    introduction(&context);
    let scenarios_repo = ScenariosRepository::new(
        command_args.scenario,
        command_args.tag,
        command_args.report_unstable,
        command_args.print_panics,
    );
    let scenario_suite_result = scenarios_repo.run(&context);
    println!("{}", scenario_suite_result.result_string());

    if command_args.set_exit_code {
        std::process::exit(if scenario_suite_result.is_failed() {
            1
        } else {
            0
        });
    }
}

fn introduction<R: rand_core::RngCore>(context: &Context<R>) {
    println!(
        r###"
        ---_ ......._-_--.
       (|\ /      / /| \  \               _  ___  ____  __  __ _   _ _   _  ____    _    _   _ ____  ____
       /  /     .'  -=-'   `.            | |/ _ \|  _ \|  \/  | | | | \ | |/ ___|  / \  | \ | |  _ \|  _ \
      /  /    .'             )        _  | | | | | |_) | |\/| | | | |  \| | |  _  / _ \ |  \| | | | | |_) |
    _/  /   .'        _.)   /        | |_| | |_| |  _ <| |  | | |_| | |\  | |_| |/ ___ \| |\  | |_| |  _ <
   /   o  o       _.-' /  .'          \___/ \___/|_| \_\_|  |_|\___/|_| \_|\____/_/   \_\_| \_|____/|_| \_\
   \          _.-'    / .'#|
    \______.-'//    .'.' \#|         SCENARIO TEST SUITE
     \|  \ | //   .'.' _ |#|
      `   \|//  .'.'_._._|#|
       .  .// .'.' | _._ \#|
       \`-|\_/ /    \ _._ \#\
        `/'\__/      \ _._ \#\
       /^|            \ _-_ \#
      '  `             \ _-_ \
                        \_

 {}jormungandr: {}
 {}jcli:        {}
 {}seed:        {}

###############################################################################
    "###,
        *style::icons::jormungandr,
        style::binary.apply_to(context.jormungandr().to_string_lossy()),
        *style::icons::jcli,
        style::binary.apply_to(context.jcli().to_string_lossy()),
        *style::icons::seed,
        style::seed.apply_to(context.seed()),
    )
}
