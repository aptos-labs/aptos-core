// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Debug, Parser, Clone, Default)]
pub struct FilterOptions {
    /// If specified, only run tests containing this string in their names
    #[clap(long, short)]
    pub filter: Option<String>,

    /// Only execute the test module/test function if their `<module_name>[::<fn_name>]` fq name exactly matches provided `--filter` value
    #[clap(long, hide = true)]
    pub exact: bool,
}

impl FilterOptions {
    pub fn matches(&self, fullname: &str) -> bool {
        match self.filter.as_ref() {
            None => true,
            Some(filter_s) => {
                if self.exact {
                    fullname == filter_s
                } else {
                    fullname.contains(filter_s)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_default_matches_everything() {
        let opts = FilterOptions::default();
        assert!(opts.matches("mod"));
        assert!(opts.matches("mod::fun"));
        assert!(opts.matches("1122"));
    }

    #[test]
    fn test_filter_with_name() {
        let opts = FilterOptions {
            filter: Some("mod".into()),
            exact: false,
        };
        assert!(opts.matches("mod"));
        assert!(opts.matches("other_mod"));
        assert!(opts.matches("main::mod"));
        assert!(opts.matches("mod::fun"));

        assert!(!opts.matches("other"));
        assert!(!opts.matches("0x1"));
    }

    #[test]
    fn test_filter_with_exact() {
        let opts = FilterOptions {
            filter: Some("mod".into()),
            exact: true,
        };
        assert!(opts.matches("mod"));

        assert!(!opts.matches("other_mod"));
        assert!(!opts.matches("mod::fun"));
        assert!(!opts.matches("main::mod"));
    }
}
