use clams::prelude::*;
use clams_bin::netatmo::*;
use failure::{Error, format_err};
use structopt::StructOpt;

const DEFAULT_CONFIG_FILE_NAME: &str = "netatmo.conf";

#[derive(StructOpt, Debug)]
#[structopt(name = "netatmo_yday",
    about = "Get Netatmo weather station data from yesterday",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    /// Config file
    #[structopt(short = "c", long = "config")]
    config_file: Option<String>,
    /// do not use colored output
    #[structopt(long = "no-color")]
    no_color: bool,
    /// Silencium
    #[structopt(short = "s", long = "silent")]
    silent: bool,
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbosity: u64,
}

fn run(args: Args) -> Result<(), Error> {
    let mut config_locations = default_locations(DEFAULT_CONFIG_FILE_NAME);
    if let Some(config) = args.config_file {
        config_locations.insert(0, config.into());
    }
    let config = NetatmoConfig::smart_load(&config_locations)
        .map_err(|e| format_err!("Failed to load config file because {}", e.to_string()))?;
    debug!("config = {:#?}", config);

    Ok(())
}

fn main() {
    let args = Args::from_args();
    clams::console::set_color(!args.no_color);

    let name = Args::clap().get_name().to_owned();

    let level: Level = args.verbosity.into();
    if !args.silent {
        eprintln!(
            "{} version={}, log level={:?}",
            name,
            env!("CARGO_PKG_VERSION"),
            &level
        );
    }

    let log_config = LogConfig::new(
        std::io::stderr(),
        !args.no_color,
        Level(log::LevelFilter::Error),
        vec![ModLevel {
            module: name.to_owned(),
            level,
        }],
        None,
    );

    init_logging(log_config).expect("Failed to initialize logging");

    match run(args) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed:");
            for c in e.iter_chain() {
                println!("{}", c);
            }
        }
    }
}
