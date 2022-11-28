use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use anyhow::{anyhow, Context, ensure};
use async_trait::async_trait;
use clap::Parser;
use convert_case::{Case, Casing};
use path_absolutize::Absolutize;
use tera::Tera;
use url::Url;
use walkdir::WalkDir;

use crate::common::types::{CliCommand, CliError, CliTypedResult, PromptOptions};
use crate::move_tool::FrameworkPackageArgs;

#[derive(Clone)]
pub enum Template {
    Default(String),
    GitUrl(Url),
}

pub fn parse_template(val: &str) -> anyhow::Result<Template> {
    if val == "empty" || val == "coin" || val == "dapp" {
        return Ok(Template::Default(val.to_string()))
    }
    if !val.starts_with("http") {
        // is not a url, fail with unknown template
        return Err(anyhow!("choose one of 'empty', 'coin', 'dapp'."))
    }
    let parsed_url = Url::parse(val);
    match parsed_url {
        Ok(url) => Ok(Template::GitUrl(url)),
        Err(_) => {
            Err(anyhow!("Invalid git url {}", val))
        }
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
                let templates_root_path = git_download_aptos_templates()?;
                let template_path = templates_root_path.join(template_name);
                self.render_tera_template(template_path)?;
            },
            Template::GitUrl(_) => unimplemented!()
        }
        Ok(())
    }
}

impl NewPackage {
    fn package_name(&self) -> anyhow::Result<String> {
        let package_name = match &self.name {
            Some(name) => name.clone(),
            None => self.package_dir.to_package_name()
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

    fn render_tera_template(&self, template_path: PathBuf) -> anyhow::Result<()> {
        let package_dir = self.package_dir.as_ref();
        let package_name = self.package_name()?;

        let tera_template = Tera::new(&format!(
            "{}/**/*",
            template_path.to_string_lossy(),
        ))
            .map_err(|_| CliError::UnexpectedError("tera error".to_string()))?;
        let tera_context = tera::Context::from_serialize(
            [
                ("package_name".to_string(), package_name),
                // ("package_lowercase_name".to_string(), package_lowercase_name),
            ]
                .into_iter()
                .collect::<HashMap<String, String>>(),
        )
            .map_err(|err| anyhow!("Tera context: {err}"))?;

        for (from, to, subpath) in tera_walk_dir(&template_path, package_dir) {
            if to.exists() {
                continue;
            }
            if to.file_name().unwrap() == ".gitkeep" {
                continue;
            }

            if from.is_dir() {
                fs::create_dir(to).map_err(|err| anyhow!("Create dir: {err}. {from:?}"))?;
                continue;
            }

            let r = tera_template
                .render(&subpath, &tera_context)
                .map_err(|err| anyhow!("Tera render: {err}."))?;
            fs::write(to, r).map_err(|err| anyhow!("{err}. {subpath}"))?;
        }
        Ok(())
    }
}

const GIT_TEMPLATE: &str = "https://github.com/mkurnikov/aptos-templates.git";
const GIT_COMMIT: &str = "540b78598d74152fbc6cb6ac6c7b139c34114259";

fn git_download_aptos_templates() -> anyhow::Result<PathBuf> {
    // TODO: download more reliably (i.e. directory exists)
    let tmp_dir = std::env::temp_dir().join(&format!("aptos_templates_{GIT_COMMIT}"));
    if !tmp_dir.exists() {
        println!("Downloading: {GIT_TEMPLATE}");
        Command::new("git")
            .arg("clone")
            .arg(GIT_TEMPLATE)
            .output()
            .context("Failed to find merge base")?;
    }

    Ok(tmp_dir)
}

fn tera_walk_dir<'a>(
    from_dir: &'a Path,
    to_dir: &'a Path,
) -> impl Iterator<Item=(PathBuf, PathBuf, String)> + 'a {
    let from_str = from_dir.to_string_lossy().to_string();

    WalkDir::new(from_dir)
        .into_iter()
        .filter_map(|path| path.ok())
        .map(|path| path.into_path())
        // .skip(1)
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

    fn from_str(path: &str) -> std::result::Result<Self, Self::Err> {
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
