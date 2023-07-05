// Copyright © Aptos Foundation

use anyhow::{format_err, Result};
use rand::{seq::SliceRandom, Rng, rngs::ThreadRng};

/// Split out the hard to test portion from the testable portion
pub fn generate_random_namespace() -> Result<String> {
    let mut rng: ThreadRng = rand::thread_rng();
    // Lets pick some four letter words ;)
    let words = random_word::all_len(4)
        .ok_or_else(|| {
            format_err!(
                "Failed to get namespace, rerun with --namespace <namespace>"
            )
        })?
        .to_vec()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    random_namespace(words, &mut rng)
}

/// Make an easy to remember random namespace for your testnet
pub fn random_namespace<R: Rng>(dictionary: Vec<String>, rng: &mut R) -> Result<String> {
    // Pick four random words
    let random_words = dictionary
        .choose_multiple(rng, 4)
        .cloned()
        .collect::<Vec<String>>();
    Ok(format!("forge-{}", random_words.join("-")))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_random_namespace() {
        let mut rng = rand::rngs::mock::StepRng::new(100, 1);
        let words = ["apple", "banana", "carrot", "durian", "eggplant", "fig"]
            .to_vec()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let namespace = random_namespace(words, &mut rng).unwrap();
        assert_eq!(namespace, "forge-durian-eggplant-fig-apple");
    }
}
