use clap::{crate_authors, crate_description, crate_name, crate_version, Command};

fn main() {
    let matches = Command::new(crate_name!())
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .get_matches();

    println!("Hello world")
}
