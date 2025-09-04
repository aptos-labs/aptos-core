// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context};
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream},
};
use move_docgen::OutputFormat;
use move_model::model::GlobalEnv;
use std::{path::PathBuf, sync::Mutex};

#[derive(Debug, Clone, clap::Parser, serde::Serialize, serde::Deserialize, Default)]
pub struct DocgenOptions {
    /// Whether to include private declarations and implementations into the generated
    /// documentation. Defaults to false.
    #[clap(long)]
    pub include_impl: bool,

    /// Whether to include specifications in the generated documentation. Defaults to false.
    #[clap(long)]
    pub include_specs: bool,

    /// Whether specifications should be put side-by-side with declarations or into a separate
    /// section. Defaults to false.
    #[clap(long)]
    pub specs_inlined: bool,

    /// Whether to include a dependency diagram. Defaults to false.
    #[clap(long)]
    pub include_dep_diagram: bool,

    /// Whether details should be put into collapsed sections. This is not supported by
    /// all markdown, but the github dialect. Defaults to false.
    #[clap(long)]
    pub collapsed_sections: bool,

    /// Package-relative path to an optional markdown template which is a used to create a
    /// landing page. Placeholders in this file are substituted as follows: `> {{move-toc}}` is
    /// replaced by a table of contents of all modules; `> {{move-index}}` is replaced by an index,
    /// and `> {{move-include NAME_OF_MODULE_OR_SCRIP}}` is replaced by the full
    /// documentation of the named entity. (The given entity will not longer be placed in
    /// its own file, so this can be used to create a single manually populated page for
    /// the package.)
    #[clap(long)]
    pub landing_page_template: Option<String>,

    /// Package-relative path to a file whose content is added to each generated markdown file.
    /// This can contain common markdown references fpr this package (e.g. `[move-book]: <url>`).
    #[clap(long)]
    pub references_file: Option<String>,

    /// Choose the output format
    #[clap(long)]
    pub output_format: Option<OutputFormat>,
}

impl DocgenOptions {
    pub fn run(
        &self,
        package_path: String,
        doc_path: Vec<String>,
        model: &GlobalEnv,
    ) -> anyhow::Result<()> {
        // To get relative paths right, we need to run docgen with relative paths. To this
        // end we need to set the current directory of the process. This in turn is not thread
        // safe, so we need to make a critical section out of the entire generation process.
        // TODO: fix this in docgen
        static MUTEX: Mutex<u8> = Mutex::new(0);
        let _lock = MUTEX.lock();
        let current_dir = std::env::current_dir()?.canonicalize()?;
        std::env::set_current_dir(&package_path)?;
        let output_directory = PathBuf::from("doc");
        let doc_path = doc_path
            .into_iter()
            .filter_map(|s| {
                PathBuf::from(s)
                    .strip_prefix(&package_path)
                    .map(|p| p.display().to_string())
                    .ok()
            })
            .collect();
        let options = move_docgen::DocgenOptions {
            section_level_start: 1,
            include_private_fun: self.include_impl,
            include_specs: self.include_specs,
            specs_inlined: self.specs_inlined,
            include_impl: self.include_impl,
            toc_depth: 3,
            collapsed_sections: self.collapsed_sections,
            output_directory: output_directory.display().to_string(),
            doc_path,
            root_doc_templates: self
                .landing_page_template
                .as_ref()
                .map(|s| vec![s.clone()])
                .unwrap_or_default(),
            references_file: self.references_file.clone(),
            include_dep_diagrams: self.include_dep_diagram,
            include_call_diagrams: false,
            compile_relative_to_output_dir: false,
            output_format: self.output_format,
            ensure_unix_paths: true,
        };
        let output = move_docgen::Docgen::new(model, &options).gen();
        if model.diag_count(Severity::Warning) > 0 {
            let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
            model.report_diag(&mut error_writer, Severity::Warning);
        }
        let res = if model.has_errors() {
            Err(anyhow!("documentation generation failed"))
        } else {
            // Write the generated output files
            std::fs::create_dir_all(&output_directory)?;
            for (file_name, content) in output {
                let dest = PathBuf::from(file_name);
                std::fs::write(dest.as_path(), content)
                    .with_context(|| format!("writing `{}`", dest.display()))?;
            }
            Ok(())
        };
        std::env::set_current_dir(current_dir)?;
        res
    }
}
