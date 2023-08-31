use clap::{Arg, ArgMatches, Command};

pub fn parse_arguments<'a>() -> ArgMatches {
    Command::new("Lambda zkEVM")
        .version("0.1")
        .author("Dompute Team")
        .about("Computing without EVM storage")
        .subcommand(
            Command::new("prove")
                .about("Performs the 'prove' operation")
                .arg(Arg::new("root").long("root").value_name("BOOL"))
                .arg(Arg::new("actual").long("actual").value_name("BOOL"))
                .arg(Arg::new("gv").long("groth16_verifier").value_name("BOOL")),
        )
        .subcommand(
            Command::new("verify")
                .about("Performs the 'verify' operation")
                .arg(
                    Arg::new("foo")
                        .long("foo")
                        .help("Specifies the value of foo"),
                ),
        )
        .subcommand(
            Command::new("dry-run")
                .about("Performs the 'dry-run' operation")
                .arg(
                    Arg::new("foo")
                        .long("foo")
                        .help("Specifies the value of foo"),
                ),
        )
        .get_matches()
}
