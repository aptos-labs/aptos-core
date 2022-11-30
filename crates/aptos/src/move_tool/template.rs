use std::collections::BTreeMap;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, ensure, Context};
use async_trait::async_trait;
use clap::Parser;
use convert_case::{Case, Casing};
use handlebars::Handlebars;
use path_absolutize::Absolutize;
use regex::Regex;
use serde_json::json;

use walkdir::WalkDir;

use crate::common::types::{CliCommand, CliTypedResult, PromptOptions};
use crate::move_tool::FrameworkPackageArgs;

#[derive(Clone)]
pub enum Template {
    Default(String),
    GitUrl(String),
}

pub fn parse_template(val: &str) -> anyhow::Result<Template> {
    let re = Regex::new("^[a-zA-Z_]+$").unwrap();
    if re.is_match(val) {
        if val == "empty" || val == "coin" || val == "dapp" {
            Ok(Template::Default(val.to_string()))
        } else {
            Err(anyhow!("choose one of 'empty', 'coin', 'dapp'."))
        }
    } else {
        Ok(Template::GitUrl(val.to_string()))
    }
}

#[derive(Parser)]
#[clap(verbatim_doc_comment)]
pub struct NewPackage {
    /// Directory to create the new Move package
    /// The folder name can be used as the package name.
    /// If the directory does not exist, it will be created
    ///
    /// Example:
    /// my_project
    /// ~/path/to/my_new_package
    /// /tmp/project_1
    #[clap(verbatim_doc_comment, value_parser)]
    pub(crate) package_dir: PackageDir,

    /// Name of the new Move package
    #[clap(long, display_order = 1)]
    pub(crate) name: Option<String>,

    /// Package template.
    /// Can be one of the default ones: 'empty', 'coin' or 'dapp',
    /// or the url of the git repository to be used as template.
    #[clap(short, long, default_value = "empty", value_parser = parse_template, display_order = 2)]
    pub(crate) template: Template,

    #[clap(flatten)]
    pub(crate) framework_package_args: FrameworkPackageArgs,
}

#[async_trait]
impl CliCommand<()> for NewPackage {
    fn command_name(&self) -> &'static str {
        "NewPackage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        match &self.template {
            Template::Default(template_name) => {
                if template_name == "empty" {
                    self.render_empty_template()?;
                    return Ok(());
                }
                let core_templates_dir =
                    std::env::temp_dir().join(&format!("aptos_templates_{GIT_COMMIT}"));

                git_download_default_templates(&core_templates_dir)?;

                let core_template_path = core_templates_dir.join(template_name);
                self.render_template_dir(core_template_path)?;
            }
            Template::GitUrl(url) => {
                let git_url = url.as_str();
                let directory_name = url_to_file_name(git_url);
                let time_millis = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let custom_template_dir =
                    std::env::temp_dir().join(format!("{}_{}", directory_name, time_millis));

                git_download_custom_template(&custom_template_dir, git_url)?;

                self.render_template_dir(custom_template_dir)?;
            }
        }
        Ok(())
    }
}

impl NewPackage {
    fn package_name(&self) -> anyhow::Result<String> {
        let package_name = match &self.name {
            Some(name) => name.clone(),
            None => self.package_dir.to_package_name(),
        };
        Ok(package_name)
    }

    fn render_empty_template(&self) -> anyhow::Result<()> {
        let package_dir = self.package_dir.as_ref();
        let package_name = self.package_name()?;

        self.framework_package_args.init_move_dir(
            package_dir,
            &package_name,
            BTreeMap::default(),
            PromptOptions::default(),
        )?;
        fs::create_dir(package_dir.join("tests"))?;
        Ok(())
    }

    fn render_template_dir(&self, template_dir_path: PathBuf) -> anyhow::Result<()> {
        let package_dir = self.package_dir.as_ref();
        let package_name = self.package_name()?;

        let dot_git_path = template_dir_path.join(".git");
        let template_fs_items = WalkDir::new(&template_dir_path)
            .into_iter()
            .filter_entry(move |entry| entry.path() != dot_git_path)
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.into_path());

        let renderer = Handlebars::new();
        let template_context = json!({ "package_name": package_name });

        let start_dir_pat = template_dir_path.to_str().unwrap();
        for source_path in template_fs_items {
            let subpath = source_path
                .to_string_lossy()
                .to_string()
                .trim_start_matches(&start_dir_pat)
                .trim_matches('/')
                .to_string();

            let dest_path = package_dir.join(subpath);
            if dest_path.exists() {
                continue;
            }

            if source_path.is_dir() {
                fs::create_dir(dest_path)
                    .map_err(|err| anyhow!("Create dir: {err}. {source_path:?}"))?;
                continue;
            }

            if let Some(ext) = source_path.extension() {
                if ext == "move" || ext == "toml" {
                    // interpolate
                    let contents = fs::read_to_string(source_path)?;
                    let rendered_contents =
                        renderer.render_template(&contents, &template_context)?;
                    fs::write(dest_path, rendered_contents)?;
                    continue;
                }
            }
            // copy as-is
            fs::copy(source_path, dest_path)?;
        }
        Ok(())
    }
}

const GIT_APTOS_TEMPLATES_URL: &str = "https://github.com/mkurnikov/aptos-templates.git";
const GIT_COMMIT: &str = "5a7c26311c8c406ca00dfa08fc4cd4cbc5d66268";

fn git_download_default_templates(tmp_dir: &PathBuf) -> anyhow::Result<()> {
    // TODO: download more reliably (i.e. directory exists)
    if !tmp_dir.exists() {
        println!("Downloading: {GIT_APTOS_TEMPLATES_URL}");
        let output = Command::new("git")
            .arg("clone")
            .arg(GIT_APTOS_TEMPLATES_URL)
            .arg(tmp_dir)
            .output()
            .context("Failed to find merge base")?;
        if output.status.code() != Some(0) {
            eprintln!("{}", String::from_utf8(output.stderr)?);
            return Err(anyhow!("Clone failed"));
        }
        let tmp_dir_str = tmp_dir.to_string_lossy().to_string();
        Command::new("git")
            .args(["-C", &tmp_dir_str, "checkout", GIT_COMMIT])
            .output()
            .map_err(|_| anyhow::anyhow!("Failed to checkout Git reference '{}'", GIT_COMMIT,))?;
    }
    Ok(())
}

fn git_download_custom_template(tmp_dir: &PathBuf, git_url: &str) -> anyhow::Result<()> {
    // TODO: download more reliably (i.e. directory exists)
    if !tmp_dir.exists() {
        println!("Downloading: {git_url}");
        let output = Command::new("git")
            .arg("clone")
            .arg(git_url)
            .arg(tmp_dir)
            .args(["--depth", "1"])
            .output()
            .context("Failed to find merge base")?;
        if output.status.code() != Some(0) {
            eprintln!("{}", String::from_utf8(output.stderr)?);
            return Err(anyhow!("Clone failed"));
        }
    }
    Ok(())
}

fn tera_walk_dir<'a>(
    from_dir: &'a Path,
    to_dir: &'a Path,
) -> impl Iterator<Item = (PathBuf, PathBuf, String)> + 'a {
    let from_str = from_dir.to_string_lossy().to_string();

    let dot_git_path = from_dir.join(".git");
    WalkDir::new(from_dir)
        .into_iter()
        .filter_entry(move |entry| entry.path() == dot_git_path)
        .filter_map(|path| path.ok())
        .map(|path| path.into_path())
        .map(move |path| {
            let sub = path
                .to_string_lossy()
                .to_string()
                .trim_start_matches(&from_str)
                .trim_matches('/')
                .to_string();
            (path, to_dir.join(&sub), sub)
        })
}

fn url_to_file_name(url: &str) -> String {
    Regex::new(r"[/:.@]")
        .unwrap()
        .replace_all(url, "_")
        .to_string()
}

#[derive(Clone)]
pub struct PackageDir(PathBuf);

impl PackageDir {
    pub fn to_package_name(&self) -> String {
        self.0
            .file_name()
            .map(|name| name.to_string_lossy().to_case(Case::UpperCamel))
            .unwrap_or_default()
    }
}

impl AsRef<Path> for PackageDir {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl From<PackageDir> for PathBuf {
    fn from(value: PackageDir) -> PathBuf {
        value.0
    }
}

impl FromStr for PackageDir {
    type Err = anyhow::Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let package_dir = PathBuf::from(path).absolutize()?.to_path_buf();

        if !package_dir.exists() {
            return Ok(PackageDir(package_dir));
        }

        let is_empty = package_dir
            .read_dir()
            .map_err(|_| anyhow!("Couldn't read the directory {package_dir:?}"))?
            .filter_map(|item| item.ok())
            .next()
            .is_none();
        ensure!(is_empty, "The directory is not empty {package_dir:?}");

        Ok(PackageDir(package_dir))
    }
}
