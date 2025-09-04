// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use velor_dkg::pvss::{
    test_utils::get_weighted_configs_for_benchmarking, traits::SecretSharingConfig,
};
use rand::thread_rng;

#[ignore]
#[test]
fn print_best_worst_avg_case_subsets() {
    let wcs = get_weighted_configs_for_benchmarking();

    let mut rng = thread_rng();

    for wc in wcs {
        println!("{wc}");
        for i in 0..wc.get_total_num_players() {
            print!("p[{i}]: {}, ", wc.get_player_weight(&wc.get_player(i)));
        }
        println!();

        println!(
            "Average case subset size: {}",
            wc.get_average_size_of_eligible_subset(1000, &mut rng)
        );

        let worst_case = wc.get_worst_case_eligible_subset_of_players(&mut rng);
        println!(
            "Worst case subset is of size {}. Player IDs are {:?}",
            worst_case.len(),
            worst_case
                .iter()
                .map(|p| p.get_id())
                .collect::<Vec<usize>>()
        );

        let best_case = wc.get_best_case_eligible_subset_of_players(&mut rng);
        println!(
            "Best case subset is of size {}. Player IDs are {:?}",
            best_case.len(),
            best_case.iter().map(|p| p.get_id()).collect::<Vec<usize>>()
        );

        println!();
    }
}
