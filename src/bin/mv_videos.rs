extern crate clams;
extern crate clams_bin;
extern crate colored;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate fern;
#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt;
extern crate subprocess;

use clams::{fs, logging};
use clams_bin::mv_videos;
use colored::Colorize;
use failure::{Error, ResultExt};
use std::path::PathBuf;
use structopt::StructOpt;
use subprocess::{Exec, Redirection};

#[derive(StructOpt, Debug)]
#[structopt(name = "mv_videos",
about = "Move video files from a nested directory structure into another, flat directory",
raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    /// File extensions to consider
    #[structopt(short = "e", long = "extension", default_value = "avi,mkv,mp4")]
    extensions: String,
    /// Only consider files bigger than this
    #[structopt(short = "s", long = "size", default_value = "100M")]
    size: String,
    /// Source directories
    #[structopt(raw(required = "true", index = "1"))]
    sources: Vec<String>,
    /// Destination directory
    #[structopt(raw(index = "2"))]
    destination: String,
    /// Only show what would be done
    #[structopt(short = "d", long = "dry")]
    dry: bool,
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbosity: u64,
}

fn run(args: Args) -> Result<(), Error> {
    let _ = mv_videos::check_size_arg(&args.size)?;
    if !PathBuf::from(&args.destination).is_dir() {
        return Err(format_err!("Destination directory '{}' does not exist.", args.destination));
    }
    let source_directories: Vec<_> = args.sources
        .iter()
        .map(|s| s.as_ref())
        .collect();
    let extensions = mv_videos::parse_extensions(&args.extensions)?;

    let find = mv_videos::build_find_cmd(&source_directories, &args.size, extensions.as_slice())?;
    debug!("find = {}", find);

    let res = Exec::shell(&find)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .capture().context(format!("Failed to spawn shell command: '{}'", find))?;
    if !res.exit_status.success() {
        return Err(format_err!("Shell command failed: '{}', because:\n{}", find, res.stdout_str()));
    }
    let files: Vec<_> = res.stdout_str()
        .lines()
        .map(|f| PathBuf::from(f))
        .collect();
    debug!("found files = {:#?}", files);

    let (files, non_existing): (Vec<_>, Vec<_>) = files
        .into_iter()
        .partition(|f| fs::file_exists(&f) );
    debug!("non existing files = {:#?}", non_existing);

    if !non_existing.is_empty() {
        return Err(format_err!("Could not find files returned from find command {:#?}", non_existing));
    }

    let moves: Vec<(_,_)> = files
        .iter()
        .map(|f| {
            let dest_path = mv_videos::destination_path(&args.destination, f).unwrap();
            (f, dest_path)
        })
        .collect();

    for (from, to) in moves {
        // Safe unwrap because we already checked the paths.
        print!("Moving {} to {} ...", from.to_str().unwrap().yellow(), to.to_str().unwrap().yellow());
        println!(" {}.", "done".green());
    }

    Ok(())
}

fn main() {
    let args = Args::from_args();

    let log_level = logging::int_to_log_level(args.verbosity);
    logging::init_logging("mv_videos", log_level, log::LevelFilter::Warn).expect("Failed to initialize logging");

    println!("{} {}, log level={}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), log_level);
    debug!("args = {:#?}", args);

    match run(args) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed:");
            for c in e.causes() {
                println!("{}", c);
            }
        }
    }
}
