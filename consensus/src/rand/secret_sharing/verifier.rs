// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::SECRET_SHARE_BAD_SHARES;
use anyhow::ensure;
use aptos_logger::warn;
use aptos_types::secret_sharing::{Author, SecretShare, SecretShareConfig};
use dashmap::DashSet;
use rayon::prelude::*;
use std::collections::HashMap;

pub struct SecretShareVerifier {
    config: SecretShareConfig,
    optimistic: bool,
    pessimistic_set: DashSet<Author>,
}

impl SecretShareVerifier {
    pub fn new(config: SecretShareConfig, optimistic: bool) -> Self {
        Self {
            config,
            optimistic,
            pessimistic_set: DashSet::new(),
        }
    }

    pub fn config(&self) -> &SecretShareConfig {
        &self.config
    }

    fn should_verify_optimistically(&self, author: &Author) -> bool {
        self.optimistic && !self.pessimistic_set.contains(author)
    }

    fn add_to_pessimistic_set(&self, author: Author) {
        self.pessimistic_set.insert(author);
    }

    fn verify_structural(&self, author: &Author) -> anyhow::Result<()> {
        let _index = self.config.get_id(author)?;
        Ok(())
    }

    pub fn optimistic_verify(&self, share: &SecretShare, sender: &Author) -> anyhow::Result<()> {
        ensure!(
            share.author() == sender,
            "Author {} does not match sender {}",
            share.author(),
            sender
        );
        if self.should_verify_optimistically(share.author()) {
            self.verify_structural(share.author())
        } else {
            share.verify(&self.config)
        }
    }

    /// Individually verify all shares and remove any that fail cryptographic
    /// verification.  Bad authors are added to the pessimistic set so future
    /// shares from them are fully verified on ingress.
    ///
    /// When `optimistic` is false, all shares were already fully verified on
    /// ingress, so this is a no-op.
    pub fn evict_bad_shares(&self, shares: &mut HashMap<Author, SecretShare>) {
        if !self.optimistic {
            return;
        }
        let bad_authors: Vec<Author> = shares
            .values()
            .par_bridge()
            .filter(|s| s.verify(&self.config).is_err())
            .map(|s| *s.author())
            .collect();
        for author in &bad_authors {
            warn!("Share from {} failed individual verification", author);
            SECRET_SHARE_BAD_SHARES
                .with_label_values(&[&author.short_str_lossless()])
                .inc();
            self.add_to_pessimistic_set(*author);
            shares.remove(author);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::test_utils::{
        create_bad_secret_share, create_metadata, create_secret_share, TestContext,
    };
    use std::sync::Arc;

    #[test]
    fn test_should_verify_optimistically_flag_off() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = Arc::new(SecretShareVerifier::new(
            ctx.secret_share_config.clone(),
            false,
        ));
        assert!(!verifier.should_verify_optimistically(&ctx.authors[0]));
    }

    #[test]
    fn test_should_verify_optimistically_flag_on() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = Arc::new(SecretShareVerifier::new(
            ctx.secret_share_config.clone(),
            true,
        ));
        assert!(verifier.should_verify_optimistically(&ctx.authors[0]));

        verifier.add_to_pessimistic_set(ctx.authors[0]);
        assert!(!verifier.should_verify_optimistically(&ctx.authors[0]));
        assert!(verifier.should_verify_optimistically(&ctx.authors[1]));
    }

    #[test]
    fn test_verify_structural_valid_author() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), true);
        assert!(verifier.verify_structural(&ctx.authors[0]).is_ok());
    }

    #[test]
    fn test_verify_structural_invalid_author() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), true);
        let unknown = Author::random();
        assert!(verifier.verify_structural(&unknown).is_err());
    }

    #[test]
    fn test_optimistic_verify_fast_path() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), true);
        let metadata = create_metadata(ctx.epoch, 5);
        let share = create_secret_share(&ctx, 1, &metadata);
        assert!(verifier.optimistic_verify(&share, &ctx.authors[1]).is_ok());
    }

    #[test]
    fn test_optimistic_verify_pessimistic_fallback() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = Arc::new(SecretShareVerifier::new(
            ctx.secret_share_config.clone(),
            true,
        ));
        let metadata = create_metadata(ctx.epoch, 5);
        let share = create_secret_share(&ctx, 1, &metadata);

        verifier.add_to_pessimistic_set(ctx.authors[1]);
        assert!(verifier.optimistic_verify(&share, &ctx.authors[1]).is_ok());
    }

    #[test]
    fn test_optimistic_verify_sender_mismatch() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), true);
        let metadata = create_metadata(ctx.epoch, 5);
        let share = create_secret_share(&ctx, 1, &metadata);
        assert!(verifier.optimistic_verify(&share, &ctx.authors[2]).is_err());
    }

    #[test]
    fn test_evict_bad_shares_removes_bad_and_keeps_good() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), true);
        let metadata = create_metadata(ctx.epoch, 5);

        let mut shares = HashMap::new();
        for i in 0..3 {
            let share = create_secret_share(&ctx, i, &metadata);
            shares.insert(ctx.authors[i], share);
        }
        let bad_share = create_bad_secret_share(&ctx, 3, &metadata);
        shares.insert(ctx.authors[3], bad_share);

        assert_eq!(shares.len(), 4);
        verifier.evict_bad_shares(&mut shares);

        assert_eq!(shares.len(), 3);
        assert!(!shares.contains_key(&ctx.authors[3]));
        for i in 0..3 {
            assert!(shares.contains_key(&ctx.authors[i]));
        }
        assert!(verifier.pessimistic_set.contains(&ctx.authors[3]));
        assert!(!verifier.pessimistic_set.contains(&ctx.authors[0]));
    }

    #[test]
    fn test_evict_bad_shares_noop_when_not_optimistic() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = SecretShareVerifier::new(ctx.secret_share_config.clone(), false);
        let metadata = create_metadata(ctx.epoch, 5);

        let mut shares = HashMap::new();
        let good_share = create_secret_share(&ctx, 0, &metadata);
        shares.insert(ctx.authors[0], good_share);
        let bad_share = create_bad_secret_share(&ctx, 1, &metadata);
        shares.insert(ctx.authors[1], bad_share);

        verifier.evict_bad_shares(&mut shares);
        assert_eq!(shares.len(), 2);
    }
}
