use clams::prelude::*;
use clams_bin::new_note::*;
use failure::{format_err, Error};
use std::path::PathBuf;
use structopt::StructOpt;

const DEFAULT_CONFIG_FILE_NAME: &str = "new_note.conf";

#[derive(StructOpt, Debug)]
#[structopt(
    name = "new_note",
    about = "Create new blog article or note from markdown template with frontmatter",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Args {
    /// Config file
    #[structopt(short = "c", long = "config")]
    config_file: Option<String>,
    /// title
    #[structopt(short = "t", long = "title")]
    title: String,
    /// Publication date
    #[structopt(short = "d", long = "date", default_value = "now")]
    date: String,
    /// Open new note in default editor
    #[structopt(short = "e", long = "edit")]
    edit: bool,
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
    let config = NewNoteConfig::smart_load(&config_locations)
        .map_err(|e| format_err!("Failed to load config file because {}", e.to_string()))?;
    debug!("config = {:#?}", config);

    if !PathBuf::from(&config.notes_directory).is_dir() {
        return Err(format_err!(
            "Notes directory '{}' does not exist.",
            config.notes_directory
        ));
    }

    let date = str_date_to_date(&args.date)?;

    let mut notes_path = PathBuf::from(&config.notes_directory);
    notes_path.push(date_to_iso_day(&date));
    notes_path.push(title_to_file_name(&args.title));

    if notes_path.is_file() {
        return Err(format_err!(
            "Cowardly refusing to overwrite existing file {:?}.",
            notes_path
        ));
    }

    let frontmatter = FrontMatter {
        title: args.title,
        date: date_to_iso_day(&date),
    };

    debug!("Creating note '{:?}' with title = '{}', publication date = '{}', and launching editor = '{}'", notes_path, &frontmatter.title, &frontmatter.date, args.edit);

    let res = create_note(notes_path.as_path(), &config.notes_template, &frontmatter)
        .map_err(|e| format_err!("Failed to create note because {}", e.to_string()));

    if res.is_ok() && args.edit {
        let _ = open_editor(notes_path.as_path());
    }

    res
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
