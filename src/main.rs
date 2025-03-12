use clap::{
    crate_authors, crate_description, crate_name, crate_version, Arg, ArgAction, ArgMatches,
    Command,
};
use env_logger::Builder;
use kopye::api::KopyeError;
use log::LevelFilter;
use miette::Result as MietteResult;
use std::env;

fn main() -> MietteResult<()> {
    let matches = Command::new(crate_name!())
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(
            Command::new("copy")
                .about("Copies a template from a repo reference to a destination")
                .arg(
                    Arg::new("repo")
                        .help("git repository reference where templates live")
                        .required(true),
                )
                .arg(Arg::new("template").help("template name").required(true))
                .arg(
                    Arg::new("destination")
                        .help("The destination directory where the project will be created")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("list templates")
                .arg(Arg::new("repo").help("git repository reference where templates live")),
        )
        .get_matches();

    let is_verbose = matches.get_flag("verbose");

    init_logger(is_verbose);

    match matches.subcommand() {
        Some(("copy", args)) => {
            handle_copy(args).map_err(miette::Report::new)?;

            Ok(())
        }
        Some(("list", args)) => {
            handle_list(args).map_err(miette::Report::new)?;

            Ok(())
        }
        _ => unreachable!(),
    }
}

fn init_logger(verbose: bool) {
    let mut builder = Builder::from_default_env();

    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    builder.filter(None, level);

    builder.init();
}

fn handle_copy(args: &ArgMatches) -> Result<(), KopyeError> {
    let repo = args.get_one::<String>("repo").expect("repo required");
    let template_name = args
        .get_one::<String>("template")
        .expect("template required");
    let destination = args
        .get_one::<String>("destination")
        .expect("destination expected");

    kopye::api::copy_template(repo, template_name, destination)
}

fn handle_list(args: &ArgMatches) -> Result<(), KopyeError> {
    let repo = args.get_one::<String>("repo").expect("repo required");

    kopye::api::list_templates(repo)
}
