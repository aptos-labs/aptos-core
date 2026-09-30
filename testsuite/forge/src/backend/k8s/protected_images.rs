// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{bail, Result};
use std::{collections::BTreeMap, env};

pub const VALIDATOR_TESTING_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/validator-testing";
pub const TOOLS_REPO: &str = "us-docker.pkg.dev/aptos-registry/docker/tools";

#[derive(Clone, Debug, Default)]
pub struct ProtectedImages {
    tag: Option<String>,
    digests: BTreeMap<String, String>,
}

impl ProtectedImages {
    pub fn from_env() -> Result<Self> {
        Self::from_values(
            env::var("PROTECTED_IMAGE_TAG").ok().as_deref(),
            env::var("PROTECTED_IMAGE_DIGESTS").ok().as_deref(),
        )
    }

    pub(crate) fn from_values(tag: Option<&str>, raw_digests: Option<&str>) -> Result<Self> {
        match (
            tag.filter(|tag| !tag.is_empty()),
            raw_digests.filter(|raw| !raw.is_empty()),
        ) {
            (None, None) => Ok(Self::default()),
            (Some(tag), Some(raw_digests)) => {
                let digests: BTreeMap<String, String> = serde_json::from_str(raw_digests)?;
                for (repository, digest) in &digests {
                    if !valid_digest(digest) {
                        bail!("Invalid protected image digest for {repository}");
                    }
                }
                Ok(Self {
                    tag: Some(tag.to_string()),
                    digests,
                })
            },
            _ => bail!("PROTECTED_IMAGE_TAG and PROTECTED_IMAGE_DIGESTS must be set together"),
        }
    }

    pub fn digest_for<'a>(&'a self, repository: &str, tag: &str) -> Result<Option<&'a str>> {
        if self.tag.as_deref() != Some(tag) {
            return Ok(None);
        }
        self.digests
            .get(repository)
            .map(|digest| Some(digest.as_str()))
            .ok_or_else(|| anyhow::anyhow!("Missing protected image digest for {repository}"))
    }

    pub fn chart_tag(&self, repository: &str, tag: &str) -> Result<Option<String>> {
        Ok(self
            .digest_for(repository, tag)?
            .map(|digest| format!("{tag}@{digest}")))
    }

    pub fn image_ref(&self, repository: &str, tag: &str) -> Result<String> {
        Ok(match self.digest_for(repository, tag)? {
            Some(digest) => format!("{repository}@{digest}"),
            None => format!("{repository}:{tag}"),
        })
    }
}

fn valid_digest(digest: &str) -> bool {
    digest.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_the_protected_tag() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let map = format!(r#"{{"{VALIDATOR_TESTING_REPO}":"{digest}"}}"#);
        let images = ProtectedImages::from_values(Some("approved"), Some(&map)).unwrap();
        assert_eq!(
            images
                .chart_tag(VALIDATOR_TESTING_REPO, "approved")
                .unwrap(),
            Some(format!("approved@{digest}"))
        );
        assert_eq!(
            images
                .chart_tag(VALIDATOR_TESTING_REPO, "baseline")
                .unwrap(),
            None
        );
        assert!(images.chart_tag(TOOLS_REPO, "approved").is_err());
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert!(ProtectedImages::from_values(Some("approved"), None).is_err());
        assert!(ProtectedImages::from_values(Some("approved"), Some("not json")).is_err());
        assert!(
            ProtectedImages::from_values(Some("approved"), Some(r#"{"x":"sha256:bad"}"#)).is_err()
        );
    }
}
