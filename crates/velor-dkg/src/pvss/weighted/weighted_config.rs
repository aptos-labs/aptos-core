// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::evaluation_domain::{BatchEvaluationDomain, EvaluationDomain},
    pvss::{traits, traits::SecretSharingConfig, Player, ThresholdConfig},
};
use anyhow::anyhow;
use more_asserts::assert_lt;
use rand::Rng;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Encodes the *threshold configuration* for a *weighted* PVSS: i.e., the minimum weight $w$ and
/// the total weight $W$ such that any subset of players with weight $\ge w$ can reconstruct a
/// dealt secret given a PVSS transcript.
#[allow(non_snake_case)]
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct WeightedConfig {
    /// A weighted config is a $w$-out-of-$W$ threshold config, where $w$ is the minimum weight
    /// needed to reconstruct the secret and $W$ is the total weight.
    tc: ThresholdConfig,
    /// The total number of players in the protocol.
    num_players: usize,
    /// Each player's weight
    weight: Vec<usize>,
    /// Player's starting index `a` in a vector of all `W` shares, such that this player owns shares
    /// `W[a, a + weight[player])`. Useful during weighted secret reconstruction.
    starting_index: Vec<usize>,
    /// The maximum weight of any player.
    max_weight: usize,
    /// The minimum weight of any player.
    min_weight: usize,
}

impl WeightedConfig {
    #[allow(non_snake_case)]
    /// Initializes a weighted secret sharing configuration with threshold weight `threshold_weight`
    /// and the $i$th player's weight stored in `weight[i]`.
    pub fn new(threshold_weight: usize, weights: Vec<usize>) -> anyhow::Result<Self> {
        if threshold_weight == 0 {
            return Err(anyhow!(
                "expected the minimum reconstruction weight to be > 0"
            ));
        }

        if weights.is_empty() {
            return Err(anyhow!("expected a non-empty vector of player weights"));
        }
        let max_weight = *weights.iter().max().unwrap();
        let min_weight = *weights.iter().min().unwrap();

        let n = weights.len();
        let W = weights.iter().sum();

        // e.g., Suppose the weights for players 0, 1 and 2 are [2, 4, 3]
        // Then, our PVSS transcript implementation will store a vector of 2 + 4 + 3 = 9 shares,
        // such that:
        //  - Player 0 will own the shares at indices [0..2), i.e.,starting index 0
        //  - Player 1 will own the shares at indices [2..2 + 4) = [2..6), i.e.,starting index 2
        //  - Player 2 will own the shares at indices [6, 6 + 3) = [6..9), i.e., starting index 6
        let mut starting_index = Vec::with_capacity(weights.len());
        starting_index.push(0);

        for w in weights.iter().take(n - 1) {
            starting_index.push(starting_index.last().unwrap() + w);
        }

        let tc = ThresholdConfig::new(threshold_weight, W)?;
        Ok(WeightedConfig {
            tc,
            num_players: n,
            weight: weights,
            starting_index,
            max_weight,
            min_weight,
        })
    }

    pub fn get_min_weight(&self) -> usize {
        self.min_weight
    }

    /// Returns _a_ player who has the smallest weight.
    pub fn get_min_weight_player(&self) -> Player {
        if let Some((i, _weight)) = self
            .weight
            .iter()
            .enumerate()
            .min_by_key(|&(_, &weight)| weight)
        {
            // println!("Player {} has the smallest weight: {}", i, _weight);
            self.get_player(i)
        } else {
            panic!("Weights vector should not be empty");
        }
    }

    /// Returns _a_ player who has the largest weight.
    pub fn get_max_weight_player(&self) -> Player {
        if let Some((i, _weight)) = self
            .weight
            .iter()
            .enumerate()
            .max_by_key(|&(_, &weight)| weight)
        {
            // println!("Player {} has the largest weight: {}", i, _weight);
            self.get_player(i)
        } else {
            panic!("Weights vector should not be empty");
        }
    }

    pub fn get_max_weight(&self) -> usize {
        self.max_weight
    }

    pub fn get_threshold_config(&self) -> &ThresholdConfig {
        &self.tc
    }

    pub fn get_threshold_weight(&self) -> usize {
        self.tc.t
    }

    pub fn get_total_weight(&self) -> usize {
        self.tc.n
    }

    pub fn get_player_weight(&self, player: &Player) -> usize {
        self.weight[player.id]
    }

    pub fn get_player_starting_index(&self, player: &Player) -> usize {
        self.starting_index[player.id]
    }

    /// In an unweighted secret sharing scheme, each player has one share. We can weigh such a scheme
    /// by splitting a player into as many "virtual" players as that player's weight, assigning one
    /// share per "virtual player."
    ///
    /// This function returns the "virtual" player associated with the $i$th sub-share of this player.
    pub fn get_virtual_player(&self, player: &Player, j: usize) -> Player {
        // println!("WeightedConfig::get_virtual_player({player}, {i})");
        assert_lt!(j, self.weight[player.id]);

        let id = self.get_share_index(player.id, j).unwrap();

        Player { id }
    }

    pub fn get_all_virtual_players(&self, player: &Player) -> Vec<Player> {
        let w = self.get_player_weight(player);

        (0..w)
            .map(|i| self.get_virtual_player(player, i))
            .collect::<Vec<Player>>()
    }

    /// `i` is the player's index, from 0 to `self.tc.n`
    /// `j` is the player's share #, from 0 to `self.weight[i]`
    ///
    /// Returns the index of this player's share in the vector of shares, or None if out of bounds.
    pub fn get_share_index(&self, i: usize, j: usize) -> Option<usize> {
        if j < self.weight[i] {
            Some(self.starting_index[i] + j)
        } else {
            None
        }
    }

    pub fn get_batch_evaluation_domain(&self) -> &BatchEvaluationDomain {
        &self.tc.get_batch_evaluation_domain()
    }

    pub fn get_evaluation_domain(&self) -> &EvaluationDomain {
        &self.tc.get_evaluation_domain()
    }

    /// NOTE: RNG is passed in to maintain function signature compatibility with
    /// `SecretSharingConfig::get_random_eligible_subset_of_players`, so as to easily benchmark
    /// with different methods of sampling subsets.
    pub fn get_best_case_eligible_subset_of_players<R: RngCore + CryptoRng>(
        &self,
        _rng: &mut R,
    ) -> Vec<Player> {
        let mut player_and_weights = self.sort_players_by_weight();

        self.pop_eligible_subset(&mut player_and_weights)
    }

    /// NOTE: RNG is passed in to maintain function signature compatibility with
    /// `SecretSharingConfig::get_random_eligible_subset_of_players`, so as to easily benchmark
    /// with different methods of sampling subsets.
    pub fn get_worst_case_eligible_subset_of_players<R: RngCore + CryptoRng>(
        &self,
        _rng: &mut R,
    ) -> Vec<Player> {
        let mut player_and_weights = self.sort_players_by_weight();

        player_and_weights.reverse();

        self.pop_eligible_subset(&mut player_and_weights)
    }

    pub fn get_average_size_of_eligible_subset<R: RngCore + CryptoRng>(
        &self,
        sample_size: usize,
        rng: &mut R,
    ) -> usize {
        let mut average = 0;
        for _ in 0..sample_size {
            average += self.get_random_eligible_subset_of_players(rng).len();
        }
        average / sample_size
    }

    fn sort_players_by_weight(&self) -> Vec<(Player, usize)> {
        // the set of remaining players that we are picking a "capable" subset from
        let mut player_and_weights = self
            .weight
            .iter()
            .enumerate()
            .map(|(i, w)| (self.get_player(i), *w))
            .collect::<Vec<(Player, usize)>>();

        player_and_weights.sort_by(|a, b| a.1.cmp(&b.1));
        player_and_weights
    }

    fn pop_eligible_subset(&self, player_and_weights: &mut Vec<(Player, usize)>) -> Vec<Player> {
        let mut picked_players = vec![];

        let mut current_weight = 0;
        while current_weight < self.tc.t {
            let (player, weight) = player_and_weights.pop().unwrap();

            picked_players.push(player);

            // rinse and repeat until the picked players jointly have enough weight
            current_weight += weight;
        }

        picked_players
    }
}

impl Display for WeightedConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "weighted/{}-out-of-{}/{}-players",
            self.tc.t, self.tc.n, self.num_players
        )
    }
}

impl traits::SecretSharingConfig for WeightedConfig {
    /// For testing only.
    fn get_random_player<R>(&self, rng: &mut R) -> Player
    where
        R: RngCore + CryptoRng,
    {
        Player {
            id: rng.gen_range(0, self.get_total_num_players()),
        }
    }

    fn get_random_eligible_subset_of_players<R>(&self, rng: &mut R) -> Vec<Player>
    where
        R: RngCore,
    {
        // the randomly-picked "capable" subset of players who can reconstruct the secret
        let mut picked_players = vec![];
        // the set of remaining players that we are picking a "capable" subset from
        let mut player_and_weights = self
            .weight
            .iter()
            .enumerate()
            .map(|(i, w)| (i, *w))
            .collect::<Vec<(usize, usize)>>();
        let mut current_weight = 0;

        while current_weight < self.tc.t {
            // pick a random player, and move it to the picked set
            let idx = rng.gen_range(0, player_and_weights.len());
            let (player_id, weight) = player_and_weights[idx];
            picked_players.push(self.get_player(player_id));

            // efficiently remove the picked player from the set of remaining players
            let len = player_and_weights.len();
            if len > 1 {
                player_and_weights.swap(idx, len - 1);
                player_and_weights.pop();
            }

            // rinse and repeat until the picked players jointly have enough weight
            current_weight += weight;
        }

        // println!();
        // println!(
        //     "Returned random capable subset {{ {} }}",
        //     vec_to_str!(picked_players)
        // );
        picked_players
    }

    fn get_total_num_players(&self) -> usize {
        self.num_players
    }

    fn get_total_num_shares(&self) -> usize {
        self.tc.n
    }
}

#[cfg(test)]
mod test {
    use crate::pvss::{traits::SecretSharingConfig, WeightedConfig};

    #[test]
    fn bvt() {
        // 1-out-of-1 weighted
        let wc = WeightedConfig::new(1, vec![1]).unwrap();
        assert_eq!(wc.starting_index.len(), 1);
        assert_eq!(wc.starting_index[0], 0);
        assert_eq!(wc.get_virtual_player(&wc.get_player(0), 0).id, 0);

        // 1-out-of-2, weights 2
        let wc = WeightedConfig::new(1, vec![2]).unwrap();
        assert_eq!(wc.starting_index.len(), 1);
        assert_eq!(wc.starting_index[0], 0);
        assert_eq!(wc.get_virtual_player(&wc.get_player(0), 0).id, 0);
        assert_eq!(wc.get_virtual_player(&wc.get_player(0), 1).id, 1);

        // 1-out-of-2, weights 1, 1
        let wc = WeightedConfig::new(1, vec![1, 1]).unwrap();
        assert_eq!(wc.starting_index.len(), 2);
        assert_eq!(wc.starting_index[0], 0);
        assert_eq!(wc.starting_index[1], 1);
        assert_eq!(wc.get_virtual_player(&wc.get_player(0), 0).id, 0);
        assert_eq!(wc.get_virtual_player(&wc.get_player(1), 0).id, 1);

        // 3-out-of-5, some weights are 0.
        let wc = WeightedConfig::new(1, vec![0, 0, 0, 2, 2, 2, 0, 0, 0, 3, 3, 3, 0, 0, 0]).unwrap();
        assert_eq!(
            vec![0, 0, 0, 0, 2, 4, 6, 6, 6, 6, 9, 12, 15, 15, 15],
            wc.starting_index
        );
        assert_eq!(wc.get_virtual_player(&wc.get_player(3), 0).id, 0);
        assert_eq!(wc.get_virtual_player(&wc.get_player(3), 1).id, 1);
        assert_eq!(wc.get_virtual_player(&wc.get_player(4), 0).id, 2);
        assert_eq!(wc.get_virtual_player(&wc.get_player(4), 1).id, 3);
        assert_eq!(wc.get_virtual_player(&wc.get_player(5), 0).id, 4);
        assert_eq!(wc.get_virtual_player(&wc.get_player(5), 1).id, 5);
        assert_eq!(wc.get_virtual_player(&wc.get_player(9), 0).id, 6);
        assert_eq!(wc.get_virtual_player(&wc.get_player(9), 1).id, 7);
        assert_eq!(wc.get_virtual_player(&wc.get_player(9), 2).id, 8);
        assert_eq!(wc.get_virtual_player(&wc.get_player(10), 0).id, 9);
        assert_eq!(wc.get_virtual_player(&wc.get_player(10), 1).id, 10);
        assert_eq!(wc.get_virtual_player(&wc.get_player(10), 2).id, 11);
        assert_eq!(wc.get_virtual_player(&wc.get_player(11), 0).id, 12);
        assert_eq!(wc.get_virtual_player(&wc.get_player(11), 1).id, 13);
        assert_eq!(wc.get_virtual_player(&wc.get_player(11), 2).id, 14);
    }
}
