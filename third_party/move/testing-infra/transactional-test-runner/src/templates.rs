// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use tera::{Context, Tera, Value};

const MACROS_BYTES: &[u8] = include_bytes!("macros.masm.tera");

pub struct TemplateContext {
    context: Context,
    tera: Tera,
}

impl Default for TemplateContext {
    fn default() -> Self {
        let mut context = Context::new();
        let mut tera = Tera::default();
        // Declare constants
        context.insert("integer_types", &[
            "u8", "u16", "u32", "u64", "u128", "u256",
        ]);
        // Declare functions
        tera.register_function(
            "mangle",
            |args: &HashMap<String, Value>| -> tera::Result<Value> {
                let Some(arg) = Self::get_arg1(args, "repr")?.as_str() else {
                    return Err(tera::Error::msg("expected string"));
                };
                Ok(Value::String(
                    arg.replace('|', "_F_")
                        .replace(['<', '>'], "$")
                        .replace('&', "R_")
                        .replace('+', "_A_")
                        .replace([':', ' '], "_"),
                ))
            },
        );
        // Register external templates, accessible via `{% import/include <name> %}`.
        tera.add_raw_template(
            "macros",
            std::str::from_utf8(MACROS_BYTES).expect("valid utf8 template"),
        )
        .expect("valid raw template");
        Self { context, tera }
    }
}

impl TemplateContext {
    pub fn expand(&self, text: &str) -> anyhow::Result<String> {
        let mut tera = self.tera.clone();
        tera.add_raw_template("source", text)?;
        let result = tera.render("source", &self.context)?;
        Ok(result)
    }

    fn get_arg1<'a>(args: &'a HashMap<String, Value>, key: &str) -> tera::Result<&'a Value> {
        let arg = args
            .get(key)
            .ok_or_else(|| tera::Error::msg(format!("expected `{}` argument", key)))?;
        if args.len() != 1 {
            return Err(tera::Error::msg("expected only one argument"));
        }
        Ok(arg)
    }
}
