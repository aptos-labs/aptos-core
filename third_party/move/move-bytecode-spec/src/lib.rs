extern crate proc_macro;

use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::{btree_map, BTreeMap};
use syn::{parse_macro_input, Data, DeriveInput, Meta};

/// Helper function to convert upper camel case to lower snake case.
/// This is programmed using a state machine, capable of handling edge cases like
//  `StudentIDCard -> student_id_card`.
fn upper_camel_to_lower_snake_case(s: &str) -> String {
    let mut res = String::new();

    let mut chars = s.chars();

    let mut buffer = match chars.next() {
        Some(c) => c,
        None => return res,
    };
    let mut ends_with_upper = false;

    for c in chars {
        match (buffer.is_ascii_uppercase(), c.is_ascii_uppercase()) {
            (true, true) => {
                res.push(buffer.to_ascii_lowercase());
                ends_with_upper = true;
            },
            (false, true) => {
                res.push(buffer);
                if buffer != '_' {
                    res.push('_');
                }
                ends_with_upper = false;
            },
            (true, false) => {
                if ends_with_upper {
                    res.push('_');
                }
                res.push(buffer.to_ascii_lowercase());
                ends_with_upper = true;
            },
            (false, false) => {
                res.push(buffer);
                ends_with_upper = false;
            },
        }
        buffer = c;
    }

    res.push(buffer.to_ascii_lowercase());

    res
}

fn trim_leading_indentation(input: &str) -> String {
    // Split the input into lines and collect into a vector with trailing spaces trimmed
    let lines: Vec<&str> = input.lines().map(|line| line.trim_end()).collect();
    if lines.is_empty() {
        return "".to_string();
    }

    // Find the first non-empty line
    let start = lines.iter().position(|line| !line.is_empty()).unwrap_or(0);
    // Find the last non-empty line
    let end = lines.iter().rposition(|line| !line.is_empty()).unwrap_or(0);

    // Slice the lines to remove leading and trailing empty lines
    let trimmed_lines: &[&str] = &lines[start..=end];

    // Determine the minimum indentation (number of leading spaces) across all non-empty lines
    let min_indent = trimmed_lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| line.chars().take_while(|c| *c == ' ').count())
        .min()
        .unwrap_or(0);

    // Create a new string with the leading spaces trimmed according to the minimum indentation
    let result_lines: Vec<String> = trimmed_lines
        .iter()
        .map(|line| {
            if line.len() > min_indent {
                line[min_indent..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    // Join the lines back together with newline characters
    result_lines.join("\n")
}

static KNOWN_ATTRIBUTES: Lazy<BTreeMap<&str, ()>> = Lazy::new(|| {
    [
        "name",
        "group",
        "description",
        "static_operands",
        "semantics",
        "paranoid_pre",
        "paranoid_post",
        "gas_type_creation_tier_0",
        "gas_type_creation_tier_1",
    ]
    .into_iter()
    .map(|attr_name| (attr_name, ()))
    .collect()
});

#[proc_macro_attribute]
pub fn bytecode_spec(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(tokens as DeriveInput);

    let enum_name = &input.ident;
    let data = match &mut input.data {
        Data::Enum(data) => data,
        _ => panic!("#[bytecode_spec] can only be applied to enums"),
    };

    let mut maps = Vec::new();
    for variant in &mut data.variants {
        let variant_name = variant.ident.to_string();

        let mut map_entries = BTreeMap::new();
        variant.attrs.retain(|attr| {
            if let Ok(Meta::NameValue(nv)) = attr.parse_meta() {
                if let Some(attr_name) = nv.path.get_ident() {
                    let attr_name = attr_name.to_string();
                    if KNOWN_ATTRIBUTES.contains_key(attr_name.as_str()) {
                        match nv.lit {
                            syn::Lit::Str(s) => {
                                match map_entries.entry(attr_name) {
                                    btree_map::Entry::Occupied(entry) => {
                                        panic!(
                                            "Attribute \"{}\" defined more than once.",
                                            entry.key()
                                        );
                                    },
                                    btree_map::Entry::Vacant(entry) => {
                                        entry.insert(trim_leading_indentation(&s.value()));
                                    },
                                }
                                return false;
                            },
                            _ => panic!("Invalid value. Expected string literal."),
                        }
                    }
                }
            }
            true
        });

        match map_entries.entry("name".to_string()) {
            btree_map::Entry::Occupied(_entry) => (),
            btree_map::Entry::Vacant(entry) => {
                entry.insert(upper_camel_to_lower_snake_case(&variant_name));
            },
        }

        let mut code = quote! {};
        for (attr_name, val) in map_entries {
            code.extend(quote! {
                map.insert(#attr_name.to_string(), #val.to_string());
            })
        }

        maps.push(quote! {
            {
                let mut map = std::collections::BTreeMap::new();
                #code
                map
            }
        });
    }

    let output = quote! {
        #input

        impl #enum_name {
            pub fn spec() -> Vec<std::collections::BTreeMap<String, String>> {
                vec![
                    #(#maps),*
                ]
            }
        }
    };

    output.into()
}
