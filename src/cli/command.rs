use clap::{value_parser, Arg, ArgMatches, Command};

pub fn parse_arguments<'a>() -> ArgMatches {
    Command::new("Lambda zkEVM")
        .version("0.1")
        .author("Dompute Team")
        .about("Computing without EVM storage")
        .subcommand(
            Command::new("prove")
                .about("Performs the 'prove' operation")
                .arg(
                    Arg::new("root")
                        .long("root")
                        .value_parser(value_parser!(bool))
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("actual")
                        .long("actual")
                        .value_parser(value_parser!(bool))
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("gv")
                        .long("gv")
                        .value_parser(value_parser!(bool))
                        .action(clap::ArgAction::SetTrue),
                ),
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
                    Arg::new("calldata")
                        .long("calldata")
                        .short('c')
                        .value_parser(value_parser!(String))
                        .action(clap::ArgAction::Set),
                )
                .arg(
                    Arg::new("bytecode")
                        .long("bytecode")
                        .short('b')
                        .value_parser(value_parser!(String))
                        .action(clap::ArgAction::Set),
                )
                .arg(
                    Arg::new("hardcode")
                        .long("hardcode")
                        .short('d')
                        .value_parser(value_parser!(String))
                        .action(clap::ArgAction::Set),
                )
                .arg(
                    Arg::new("file")
                        .long("file")
                        .value_parser(value_parser!(String))
                        .action(clap::ArgAction::Set),
                ),
        )
        .get_matches()
}
