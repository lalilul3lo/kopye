use clap::{
    crate_authors, crate_description, crate_name, crate_version, Arg, ArgAction, ArgMatches,
    Command,
};

// The CLI layer should only parse inputs and forward them to library code.
fn main() {
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
                .arg(Arg::new("repo").help("git repository reference where templates live"))
                .arg(
                    Arg::new("all")
                        .help("list all available templates")
                        .short('a')
                        .long("all")
                        .action(ArgAction::SetTrue),
                ), // if not provided fallback to repo that may be defined in global config
        )
        .get_matches();

    let is_verbose = matches.get_flag("verbose");

    match matches.subcommand() {
        Some(("copy", args)) => {
            handle_copy(args, is_verbose);
        }
        Some(("list", args)) => {
            handle_list(args, is_verbose);
        }
        _ => unreachable!(),
    }
}

fn handle_copy(args: &ArgMatches, is_verbose: bool) {
    let repo = args.get_one::<String>("repo").expect("repo required");
    let template_name = args
        .get_one::<String>("template")
        .expect("template required");
    let destination = args
        .get_one::<String>("destination")
        .expect("destination expected");

    if is_verbose {
        println!("executing in verbose mode");
    }

    kopye::actions::copy_template(repo, template_name, destination);
}

fn handle_list(args: &ArgMatches, is_verbose: bool) {
    let repo = args.get_one::<String>("repo").expect("repo required");

    let all_flag = args.get_flag("all");

    if is_verbose {
        println!("executing in verbose mode");
    }

    if all_flag {
        println!("Fetching all available templates")
    }

    kopye::actions::list_templates(repo);
}
