// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Self-contained HTML renderer for framework usage reports.

use crate::framework_usage_template::TEMPLATE;
use anyhow::{Context, Result};
use std::{io::Write, path::Path};
use tempfile::NamedTempFile;

pub(crate) fn write(output: &Path, report_json: &str) -> Result<()> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    anyhow::ensure!(
        parent.exists(),
        "HTML output directory {:?} does not exist",
        parent
    );

    // Prevent report data from terminating its application/json script element. The report's
    // identifiers cannot normally contain these characters, but escaping them keeps this safe if
    // the schema later gains free-form strings.
    let embedded_json = report_json
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    let html = TEMPLATE.replace("__FRAMEWORK_USAGE_REPORT__", &embedded_json);

    let mut file = NamedTempFile::new_in(parent)
        .with_context(|| format!("creating temporary HTML report in {:?}", parent))?;
    file.write_all(html.as_bytes())
        .context("writing framework usage HTML report")?;
    file.as_file()
        .sync_all()
        .context("syncing framework usage HTML report")?;
    file.persist(output)
        .map_err(|error| error.error)
        .with_context(|| format!("renaming temporary HTML report to {:?}", output))?;
    Ok(())
}
