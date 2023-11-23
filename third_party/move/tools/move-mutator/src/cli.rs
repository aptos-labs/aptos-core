use clap::{Arg, Command};
use serde::{Deserialize, Serialize};

/// Command line options for mutator
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The paths to the Move sources.
    pub move_sources: Vec<String>,
}

impl Options {
    /// Creates options from the TOML configuration source.
    pub fn from_toml(toml_source: &str) -> anyhow::Result<Options> {
        Ok(toml::from_str(toml_source)?)
    }

    /// Creates options from the TOML configuration file.
    pub fn from_toml_file(toml_file: &str) -> anyhow::Result<Options> {
        Self::from_toml(&std::fs::read_to_string(toml_file)?)
    }

    /// Creates Options struct from command line arguments.
    pub fn create_from_args(args: &[String]) -> anyhow::Result<Options> {
        let cli = Command::new("mutate")
            .version("0.1.0")
            .about("The Move Mutator")
            .author("Eiger Team")
            .arg(
                Arg::new("sources")
                    .num_args(1..)
                    .value_name("PATH_TO_SOURCE_FILE")
                    .help("the source files to mutate"),
            )
            .after_help("See `move-mutator/doc` and `README.md` for documentation.");

        // Parse the arguments. This will abort the program on parsing errors and print help.
        // It will also accept options like --help.
        let matches = cli.get_matches_from(args);

        // Initialize options.
        let get_vec = |s: &str| -> Vec<String> {
            match matches.get_many::<String>(s) {
                Some(vs) => vs.map(|v| v.to_string()).collect(),
                _ => vec![],
            }
        };

        let mut options = Options::default();

        if matches
            .get_many::<String>("sources")
            .unwrap_or_default()
            .len()
            > 0
        {
            options.move_sources = get_vec("sources");
        }

        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_from_toml() {
        let toml_str = r#"
            move_sources = ["src/main.rs", "src/lib.rs"]
        "#;

        let options = Options::from_toml(toml_str).unwrap();
        assert_eq!(options.move_sources, vec!["src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn test_from_toml_file() {
        let toml_str = r#"
            move_sources = ["src/main.rs", "src/lib.rs"]
        "#;

        let path = Path::new("test.toml");
        let mut file = File::create(&path).unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();

        let options = Options::from_toml_file(path.to_str().unwrap()).unwrap();
        assert_eq!(options.move_sources, vec!["src/main.rs", "src/lib.rs"]);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_create_from_args() {
        let args = vec![
            String::from("mutate"),
            String::from("src/main.rs"),
            String::from("src/lib.rs"),
        ];

        let options = Options::create_from_args(&args).unwrap();
        assert_eq!(options.move_sources, vec!["src/main.rs", "src/lib.rs"]);
    }
}
