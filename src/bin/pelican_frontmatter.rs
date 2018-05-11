extern crate clams;
extern crate clams_bin;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate structopt;
extern crate walkdir;

use clams::prelude::*;
use clams_bin::pelican_frontmatter;
use failure::Error;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pelican_frontmatter",
    about = "Adapt Frontmatter from pelican Wordpress import to comply to Jekyll / Gatsby Frontmatter syntax",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    /// Source folder
    #[structopt(short = "s", long = "source")]
    source: String,
    /// Destination folder
    #[structopt(short = "d", long = "destination")]
    destination: String,
    /// Destination folder
    #[structopt(short = "e", long = "extension", default_value="md")]
    file_extension: String,
    /// Only show what would be done
    #[structopt(long = "dry")]
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
        warn!( "{}", "Running in dry mode. No files will be written.".yellow());
    }

    if !PathBuf::from(&args.source).is_dir() {
        return Err(format_err!( "Source directory '{}' does not exist.", args.destination));
    }

    if !PathBuf::from(&args.destination).is_dir() {
        return Err(format_err!( "Destination directory '{}' does not exist.", args.destination));
    }

    let extension = OsStr::new(&args.file_extension);
    let paths: Vec<_> = PathBuf::from(&args.source)
        .read_dir()?
        // TODO: fix this unwrap; at least yield sensible error
        .filter(|p| p.as_ref().unwrap().path().extension() == Some(&extension) )
        .map(|p| p.unwrap().path())
        .collect();

    debug!("Adapting front matter with progess bar = {} and dry mode = {}, source = '{}', destination = '{}', and #files = {}", args.progress_bar, args.dry, args.source, args.destination, paths.len());

    if args.progress_bar {
        adapt_files_with_progress_bar(paths, PathBuf::from(&args.destination), args.dry)
    } else {
        adapt_files(paths, PathBuf::from(&args.destination), args.dry)
    }
}

fn adapt_files_with_progress_bar(paths: Vec<PathBuf>, destination: PathBuf, dry: bool) -> Result<(), Error> {
    let pb = ProgressBar::new(paths.len() as u64);
    let style = ProgressStyle::default_clams_bar();
    pb.set_style(style);

    for path in paths {
        let file_name = path.file_name().unwrap();
        let mut destination_file = destination.clone();
        destination_file.push(file_name);

        // Safe unwrap because we already checked the paths.
        pb.set_message(&format!( "Adapting {} to {} ...", path.to_str().unwrap().yellow(), destination_file.to_str().unwrap().yellow()
        ));
        if !dry {
            match pelican_frontmatter::adapt_pelican_frontmatter_in_file(path.as_path(), destination_file.as_path()) {
                Ok(_) => println!(" {}.", "done".green()),
                Err(e) => eprintln!( "Failed to adapt {} because {}", path.to_str().unwrap().red(), e),
            }
        }
        pb.inc(1);
    }
    pb.finish_with_message("done.");

    Ok(())
}

fn adapt_files(paths: Vec<PathBuf>, destination: PathBuf, dry: bool) -> Result<(), Error> {
    for path in paths {
        let file_name = path.file_name().unwrap();
        let mut destination_file = destination.clone();
        destination_file.push(file_name);

        // Safe unwrap because we already checked the paths.
        print!("Adapting {} to {} ...", path.to_str().unwrap().yellow(), destination_file.to_str().unwrap().yellow());
        if dry {
            println!(" {}", "simulated.".blue());
        } else {
            match pelican_frontmatter::adapt_pelican_frontmatter_in_file(path.as_path(), destination_file.as_path()) {
                Ok(_) => println!(" {}.", "done".green()),
                Err(e) => eprintln!( "Failed to adapt {} because {}", path.to_str().unwrap().red(), e),
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
    let md = ModLevel { module: name.clone(), level: my_log_level.clone() };
    init_logging(io::stderr(), !args.no_color, default, vec![md])
        .expect("Failed to initialize logging");

    let Level(log_level) = my_log_level;
    eprintln!( "{} version={}, log level={}", name, env!("CARGO_PKG_VERSION"), log_level);
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