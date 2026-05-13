// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(crate) fn trim_hex_address(name: &str) -> String {
    let Some((first, rest)) = name.split_once("::") else {
        return name.to_string();
    };
    let had_prefix = first.starts_with("0x");
    let hex_part = first.strip_prefix("0x").unwrap_or(first);
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) || hex_part.is_empty() {
        return name.to_string();
    }
    let trimmed = hex_part.trim_start_matches('0');
    let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
    let prefix = if had_prefix { "0x" } else { "00" };
    format!("{}{}::{}", prefix, trimmed, rest)
}

pub(crate) fn parse_source_location(location: &str) -> (Option<dap::types::Source>, i64) {
    let parts: Vec<&str> = location.rsplitn(2, ':').collect();
    if parts.len() == 2 {
        let line = parts[0].parse().unwrap_or(0);
        let path = parts[1];
        let name = std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned());
        let source = dap::types::Source {
            name,
            path: Some(path.to_string()),
            ..Default::default()
        };
        (Some(source), line)
    } else {
        (None, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_hex_address() {
        // With 0x prefix
        assert_eq!(
            trim_hex_address("0x0000000000000000000000000000000000000042::simple::test"),
            "0x42::simple::test"
        );
        assert_eq!(
            trim_hex_address("0x0000000000000000000000000000000000000001::module::func"),
            "0x1::module::func"
        );
        assert_eq!(trim_hex_address("0xABCD::foo"), "0xABCD::foo");

        // Without 0x prefix — uses 00 prefix
        assert_eq!(
            trim_hex_address(
                "0000000000000000000000000000000000000000000000000000000000000042::simple::2"
            ),
            "0042::simple::2"
        );
        assert_eq!(
            trim_hex_address("0000000000000000000000000000000000000001::module::func"),
            "001::module::func"
        );
        assert_eq!(
            trim_hex_address("0000000000000000000000000000000000000000::module::func"),
            "000::module::func"
        );

        // No :: — returned as-is
        assert_eq!(
            trim_hex_address("0x0000000000000000000000000000000000000000"),
            "0x0000000000000000000000000000000000000000"
        );
        assert_eq!(trim_hex_address("0x00ff"), "0x00ff");

        // Non-hex first segment — returned as-is
        assert_eq!(trim_hex_address("no_hex_here::func"), "no_hex_here::func");
    }

    #[test]
    fn test_parse_source_location() {
        let (src, line) = parse_source_location("/home/user/project/sources/test.move:42");
        let src = src.unwrap();
        assert_eq!(src.name.as_deref(), Some("test.move"));
        assert_eq!(
            src.path.as_deref(),
            Some("/home/user/project/sources/test.move")
        );
        assert_eq!(line, 42);

        let (src, line) = parse_source_location("test.move:1");
        let src = src.unwrap();
        assert_eq!(src.name.as_deref(), Some("test.move"));
        assert_eq!(src.path.as_deref(), Some("test.move"));
        assert_eq!(line, 1);

        let (src, line) = parse_source_location("no_colon_here");
        assert!(src.is_none());
        assert_eq!(line, 0);

        let (src, line) = parse_source_location("test.move:not_a_number");
        let src = src.unwrap();
        assert_eq!(src.path.as_deref(), Some("test.move"));
        assert_eq!(line, 0);

        let (src, line) = parse_source_location("");
        assert!(src.is_none());
        assert_eq!(line, 0);

        // rsplitn splits from the right, so C:\Users\file.move:10 works correctly
        let (src, line) = parse_source_location("C:\\Users\\file.move:10");
        let src = src.unwrap();
        assert_eq!(src.path.as_deref(), Some("C:\\Users\\file.move"));
        assert_eq!(line, 10);
    }
}
