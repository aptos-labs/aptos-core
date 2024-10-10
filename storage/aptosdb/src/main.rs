use anyhow::Result;
use aptos_db::db_debugger::validation::validate_db_data;
use clap::{Arg, Command};
use std::path::Path;

pub fn main() -> Result<()> {
    let matches = Command::new("db_validation")
        .arg(
            Arg::new("db_root_path")
                .short('d')
                .long("db-root")
                .value_parser(clap::value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("internal_indexer_db_path")
                .short('i')
                .long("internal-indexer-db")
                .value_parser(clap::value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("target_version")
                .short('t')
                .long("target-version")
                .value_parser(clap::value_parser!(u64))
                .required(true),
        )
        .get_matches();

    let db_root_path = matches.get_one::<String>("db_root_path").unwrap();
    let internal_indexer_db_path = matches
        .get_one::<String>("internal_indexer_db_path")
        .unwrap();

    let target_version = *matches.get_one::<u64>("target_version").unwrap();
    validate_db_data(
        Path::new(db_root_path),
        Path::new(internal_indexer_db_path),
        target_version,
    )?;

    Ok(())
}
