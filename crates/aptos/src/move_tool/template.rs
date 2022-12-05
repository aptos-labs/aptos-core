use std::collections::BTreeMap;

use std::fs;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;


use anyhow::{anyhow, ensure, Context};
use async_trait::async_trait;
use clap::{Parser};
use convert_case::{Case, Casing};
use handlebars::Handlebars;
use path_absolutize::Absolutize;

use serde_json::json;

use walkdir::WalkDir;

use crate::common::types::{CliCommand, CliTypedResult, PromptOptions};
use crate::move_tool::FrameworkPackageArgs;

const GIT_APTOS_TEMPLATES_URL: &str = "https://github.com/mkurnikov/aptos-core.git";
const GIT_COMMIT: &str = "333324900835e083e220f5619bfa44cdc3f774b3";

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

    /// Name of the core package template.
    #[clap(short, long, default_value = "empty", value_parser = ["empty", "coin", "dapp"])]
    pub(crate) template: String,

    #[clap(flatten)]
    pub(crate) framework_package_args: FrameworkPackageArgs,
}

#[async_trait]
impl CliCommand<()> for NewPackage {
    fn command_name(&self) -> &'static str {
        "NewPackage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        if &self.template == "empty" {
            self.render_empty_template()?;
            return Ok(());
        }

        let core_templates_dir =
            std::env::temp_dir().join(&format!("aptos_templates_{GIT_COMMIT}"));

        git_download_default_templates(&core_templates_dir)?;

        let template_dir_path = core_templates_dir
            .join("crates")
            .join("aptos")
            .join("src")
            .join("move_tool")
            .join("package_templates")
            .join(&self.template);
        self.render_template_dir(template_dir_path)?;
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

fn git_download_default_templates(tmp_dir: &PathBuf) -> anyhow::Result<()> {
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
        let output = Command::new("git")
            .args(["-C", &tmp_dir_str, "checkout", GIT_COMMIT])
            .output()
            .map_err(|_| anyhow::anyhow!("Failed to checkout Git reference '{}'", GIT_COMMIT,))?;
        if output.status.code() != Some(0) {
            eprintln!("{}", String::from_utf8(output.stderr)?);
            return Err(anyhow!("Checkout failed"));
        }
    }
    Ok(())
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
