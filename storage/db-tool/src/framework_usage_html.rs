// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Self-contained HTML renderer for framework usage reports.

use crate::framework_usage_template::TEMPLATE;
use anyhow::{Context, Result};
use std::{fs::File, io::Write, path::Path};

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

    let tmp_output = output.with_extension("tmp");
    let mut file = File::create(&tmp_output)
        .with_context(|| format!("creating temporary HTML report {:?}", tmp_output))?;
    file.write_all(html.as_bytes())
        .context("writing framework usage HTML report")?;
    file.sync_all()
        .context("syncing framework usage HTML report")?;
    std::fs::rename(&tmp_output, output)
        .with_context(|| format!("renaming HTML report {:?} to {:?}", tmp_output, output))?;
    Ok(())
}
