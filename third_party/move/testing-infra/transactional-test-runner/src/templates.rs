// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use tera::{Context, Tera};

const MACROS_BYTES: &[u8] = include_bytes!("macros.masm.tera");

pub struct TemplateContext<'a> {
    templates: Vec<(String, &'a [u8])>,
}

impl<'a> Default for TemplateContext<'a> {
    fn default() -> Self {
        let templates = vec![("macros".to_owned(), MACROS_BYTES)];
        Self { templates }
    }
}
impl<'a> TemplateContext<'a> {
    pub fn expand(&self, text: &str) -> anyhow::Result<String> {
        let mut context = Context::new();
        context.insert("integer_types", &[
            "u8", "u16", "u32", "u64", "u128", "u256",
        ]);
        let mut tera = Tera::default();
        tera.add_raw_templates(
            self.templates
                .iter()
                .map(|(name, bytes)| (name.as_str(), String::from_utf8_lossy(bytes).to_string())),
        )?;
        tera.add_raw_template("source", text)?;
        let result = tera.render("source", &context)?;
        Ok(result)
    }
}
