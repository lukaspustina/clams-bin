use clams::prelude::*;
use clams_bin::mv_files;
use failure::{format_err, Error};
use std::io;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use walkdir::WalkDir;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "mv_files",
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
    /// do not use colored output
    #[structopt(long = "no-color")]
    no_color: bool,
    /// Show progressbar
    #[structopt(short = "p", long = "progress-bar")]
    progress_bar: bool,
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbosity: u64,
}

fn run(args: Args) -> Result<(), Error> {
    if args.dry {
        warn!(
            "{}",
            "Running in dry mode. No moves will be performed.".yellow()
        );
    }

    let size = mv_files::human_size_to_bytes(&args.size)?;
    if !PathBuf::from(&args.destination).is_dir() {
        return Err(format_err!(
            "Destination directory '{}' does not exist.",
            args.destination
        ));
    }
    let extensions = mv_files::parse_extensions(&args.extensions)?;

    let source_directories: Vec<&str> = args.sources.iter().map(|s| s.as_ref()).collect();

    let dir_entries: Vec<_> = source_directories
        .into_iter()
        .map(|d| WalkDir::new(d).into_iter())
        .flat_map(|e| e)
        .collect::<Result<Vec<_>, _>>()?;

    let moves: Vec<(_, _)> = dir_entries
        .iter()
        .map(|e| e.path())
        .filter(|p| !p.is_dir())
        .filter(|p| {
            p.extension()
                .map_or(false, |x| extensions.contains(&x.to_str().unwrap()))
        })
        .filter(|p| p.metadata().map(|m| m.len() >= size).unwrap_or(false))
        .map(|p| {
            let dest_path = mv_files::destination_path(&args.destination, p).unwrap();
            (p, dest_path)
        })
        .collect();

    debug!(
        "moving with progess bar = {} and dry mode = {} and moves = ({}) {:#?}",
        args.progress_bar,
        args.dry,
        moves.len(),
        moves
    );

    if args.progress_bar {
        move_files_with_progress_bar(moves.as_slice(), args.dry)
    } else {
        move_files(moves.as_slice(), args.dry)
    }
}

fn move_files_with_progress_bar(moves: &[(&Path, PathBuf)], dry: bool) -> Result<(), Error> {
    let pb = ProgressBar::new(moves.len() as u64);
    let style = ProgressStyle::default_clams_bar();
    pb.set_style(style);

    for &(from, ref to) in moves {
        // Safe unwrap because we already checked the paths.
        pb.set_message(&format!(
            "Moving {} to {} ...",
            from.to_str().unwrap().yellow(),
            to.to_str().unwrap().yellow()
        ));
        if !dry {
            match std::fs::rename(from, to) {
                Ok(_) => {}
                Err(e) => eprintln!(
                    "Failed to move {} because {}",
                    from.to_str().unwrap().red(),
                    e
                ),
            }
        }
        pb.inc(1);
    }
    pb.finish_with_message("done.");

    Ok(())
}

fn move_files(moves: &[(&Path, PathBuf)], dry: bool) -> Result<(), Error> {
    for &(from, ref to) in moves {
        // Safe unwrap because we already checked the paths.
        print!(
            "Moving {} to {} ...",
            from.to_str().unwrap().yellow(),
            to.to_str().unwrap().yellow()
        );
        if dry {
            println!(" {}", "simulated.".blue());
        } else {
            match std::fs::rename(from, to) {
                Ok(_) => println!(" {}.", "done".green()),
                Err(e) => eprintln!(
                    "Failed to move {} because {}",
                    from.to_str().unwrap().red(),
                    e
                ),
            }
        }
    }

    Ok(())
}

fn main() {
    let args = Args::from_args();
    clams::console::set_color(!args.no_color);

    let name = Args::clap().get_name().to_owned();
    let my_log_level: Level = args.verbosity.into();

    let default = Level(log::LevelFilter::Warn);
    let md = ModLevel {
        module: name.clone(),
        level: my_log_level.clone(),
    };
    init_logging(io::stderr(), !args.no_color, default, vec![md])
        .expect("Failed to initialize logging");

    let Level(log_level) = my_log_level;
    eprintln!(
        "{} version={}, log level={}",
        name,
        env!("CARGO_PKG_VERSION"),
        log_level
    );
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
