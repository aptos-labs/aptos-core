// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Extract Move source code from Claude's fenced code-block responses.

use anyhow::anyhow;

/// Extract Move source from a Claude response.
///
/// Looks for the first ` ```move ` fenced code block, falling back to a plain ` ``` `
/// block if no language-tagged block is found.
pub fn extract_move_source(response: &str) -> anyhow::Result<String> {
    // Try ```move first.
    if let Some(content) = extract_fenced_block(response, "```move") {
        return Ok(content);
    }
    // Fallback: any ``` block.
    if let Some(content) = extract_fenced_block(response, "```") {
        return Ok(content);
    }
    Err(anyhow!(
        "Claude response does not contain a fenced code block (```move ... ```)"
    ))
}

/// Extract content from the first fenced block starting with the given marker.
fn extract_fenced_block(text: &str, marker: &str) -> Option<String> {
    let start_idx = text.find(marker)?;
    // Skip past the marker and the rest of the opening line.
    let after_marker = start_idx + marker.len();
    let content_start = text[after_marker..]
        .find('\n')
        .map(|i| after_marker + i + 1)?;
    // Find the closing ```.
    let content_end = text[content_start..]
        .find("\n```")
        .map(|i| content_start + i)?;
    Some(text[content_start..content_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_move_block() {
        let response = r#"Here is the refined source:

```move
module 0x1::test {
    fun foo(): u64 {
        42
    }
    spec foo {
        ensures result == 42;
    }
}
```

This should verify correctly."#;

        let result = extract_move_source(response).unwrap();
        assert!(result.contains("module 0x1::test"));
        assert!(result.contains("ensures result == 42"));
    }

    #[test]
    fn test_extract_plain_block_fallback() {
        let response = "Here:\n\n```\nmodule 0x1::m { }\n```\n";
        let result = extract_move_source(response).unwrap();
        assert!(result.contains("module 0x1::m"));
    }

    #[test]
    fn test_no_block_returns_error() {
        let response = "No code here.";
        assert!(extract_move_source(response).is_err());
    }
}
