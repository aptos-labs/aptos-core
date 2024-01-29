use std::cell::Cell;
use std::default::Default;
use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::{env, fs};

use thiserror::Error;

use crate::config::config_type::ConfigType;
#[allow(unreachable_pub)]
pub use crate::config::options::*;

#[macro_use]
pub mod config_type;
#[macro_use]
#[allow(unreachable_pub)]
pub mod options;

// This macro defines configuration options used in movefmt. Each option
// is defined as follows:
//
// `name: value type, default value, is stable, description;`
create_config! {
    max_width: usize, 90, true, "Maximum width of each line";
    indent_size: usize, 4, true, "Indent size";
    hard_tabs: bool, false, true, "Use tab characters for indentation, spaces for alignment";
    tab_spaces: usize, 4, true, "Number of spaces per tab";
    emit_mode: EmitMode, EmitMode::Files, true,
        "What emit Mode to use when none is supplied";
    verbose: Verbosity, Verbosity::Normal, true, "How much to information to emit to the user";
}

#[derive(Error, Debug)]
#[error("Could not output config: {0}")]
pub struct ToTomlError(toml::ser::Error);

impl PartialConfig {
    pub fn to_toml(&self) -> Result<String, ToTomlError> {
        // Non-user-facing options can't be specified in TOML
        let cloned = self.clone();
        ::toml::to_string(&cloned).map_err(ToTomlError)
    }
}

impl Config {
    /// Constructs a `Config` from the toml file specified at `file_path`.
    ///
    /// This method only looks at the provided path, for a method that
    /// searches parents for a `movefmt.toml` see `from_resolved_toml_path`.
    ///
    /// Returns a `Config` if the config could be read and parsed from
    /// the file, otherwise errors.
    pub(super) fn from_toml_path(file_path: &Path) -> Result<Config, Error> {
        let mut file = File::open(&file_path)?;
        let mut toml = String::new();
        file.read_to_string(&mut toml)?;
        Config::from_toml(&toml)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))
    }

    /// Resolves the config for input in `dir`.
    ///
    /// Searches for `movefmt.toml` beginning with `dir`, and
    /// recursively checking parents of `dir` if no config file is found.
    /// If no config file exists in `dir` or in any parent, a
    /// default `Config` will be returned (and the returned path will be empty).
    ///
    /// Returns the `Config` to use, and the path of the project file if there was
    /// one.
    pub(super) fn from_resolved_toml_path(dir: &Path) -> Result<(Config, Option<PathBuf>), Error> {
        /// Try to find a project file in the given directory and its parents.
        /// Returns the path of a the nearest project file if one exists,
        /// or `None` if no project file was found.
        fn resolve_project_file(dir: &Path) -> Result<Option<PathBuf>, Error> {
            let mut current = if dir.is_relative() {
                env::current_dir()?.join(dir)
            } else {
                dir.to_path_buf()
            };

            current = fs::canonicalize(current)?;

            loop {
                match get_toml_path(&current) {
                    Ok(Some(path)) => return Ok(Some(path)),
                    Err(e) => return Err(e),
                    _ => (),
                }

                // If the current directory has no parent, we're done searching.
                if !current.pop() {
                    break;
                }
            }

            // If nothing was found, check in the home directory.
            if let Some(home_dir) = dirs::home_dir() {
                if let Some(path) = get_toml_path(&home_dir)? {
                    return Ok(Some(path));
                }
            }

            // If none was found ther either, check in the user's configuration directory.
            if let Some(mut config_dir) = dirs::config_dir() {
                config_dir.push("movefmt");
                if let Some(path) = get_toml_path(&config_dir)? {
                    return Ok(Some(path));
                }
            }

            Ok(None)
        }

        match resolve_project_file(dir)? {
            None => Ok((Config::default(), None)),
            Some(path) => Config::from_toml_path(&path).map(|config| (config, Some(path))),
        }
    }

    pub fn from_toml(toml: &str) -> Result<Config, String> {
        let parsed: ::toml::Value = toml
            .parse()
            .map_err(|e| format!("Could not parse TOML: {}", e))?;
        let mut err = String::new();
        let table = parsed
            .as_table()
            .ok_or_else(|| String::from("Parsed config was not table"))?;
        for key in table.keys() {
            if !Config::is_valid_name(key) {
                let msg = &format!("Warning: Unknown configuration option `{key}`\n");
                err.push_str(msg)
            }
        }
        match parsed.try_into() {
            Ok(parsed_config) => {
                if !err.is_empty() {
                    eprint!("{err}");
                }
                Ok(Config::default().fill_from_parsed_config(parsed_config))
            }
            Err(e) => {
                err.push_str("Error: Decoding config file failed:\n");
                err.push_str(format!("{e}\n").as_str());
                err.push_str("Please check your config file.");
                Err(err)
            }
        }
    }
}

/// Loads a config by checking the client-supplied options and if appropriate, the
/// file system (including searching the file system for overrides).
pub fn load_config<O: CliOptions>(
    file_path: Option<&Path>,
    options: Option<O>,
) -> Result<(Config, Option<PathBuf>), Error> {
    let over_ride = match options {
        Some(ref opts) => config_path(opts)?,
        None => None,
    };

    let result = if let Some(over_ride) = over_ride {
        Config::from_toml_path(over_ride.as_ref()).map(|p| (p, Some(over_ride.to_owned())))
    } else if let Some(file_path) = file_path {
        Config::from_resolved_toml_path(file_path)
    } else {
        Ok((Config::default(), None))
    };

    result.map(|(mut c, p)| {
        if let Some(options) = options {
            options.apply_to(&mut c);
        }
        (c, p)
    })
}

// Check for the presence of known config file names (`movefmt.toml, `.movefmt.toml`) in `dir`
//
// Return the path if a config file exists, empty if no file exists, and Error for IO errors
fn get_toml_path(dir: &Path) -> Result<Option<PathBuf>, Error> {
    const CONFIG_FILE_NAMES: [&str; 2] = [".movefmt.toml", "movefmt.toml"];
    for config_file_name in &CONFIG_FILE_NAMES {
        let config_file = dir.join(config_file_name);
        match fs::metadata(&config_file) {
            // Only return if it's a file to handle the unlikely situation of a directory named
            // `movefmt.toml`.
            Ok(ref md) if md.is_file() => return Ok(Some(config_file)),
            // Return the error if it's something other than `NotFound`; otherwise we didn't
            // find the project file yet, and continue searching.
            Err(e) => {
                if e.kind() != ErrorKind::NotFound {
                    let ctx = format!("Failed to get metadata for config file {:?}", &config_file);
                    let err = anyhow::Error::new(e).context(ctx);
                    return Err(Error::new(ErrorKind::Other, err));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

fn config_path(options: &dyn CliOptions) -> Result<Option<PathBuf>, Error> {
    let config_path_not_found = |path: &str| -> Result<Option<PathBuf>, Error> {
        Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "Error: unable to find a config file for the given path: `{}`",
                path
            ),
        ))
    };

    // Read the config_path and convert to parent dir if a file is provided.
    // If a config file cannot be found from the given path, return error.
    match options.config_path() {
        Some(path) if !path.exists() => config_path_not_found(path.to_str().unwrap()),
        Some(path) if path.is_dir() => {
            let config_file_path = get_toml_path(path)?;
            if config_file_path.is_some() {
                Ok(config_file_path)
            } else {
                config_path_not_found(path.to_str().unwrap())
            }
        }
        path => Ok(path.map(ToOwned::to_owned)),
    }
}
