// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::multi_region::wrap_with_two_region_env;
use crate::{suites::ungrouped::Delay, KILOBYTE, MEGABYTE};
use velor_config::config::NetbenchConfig;
use velor_forge::ForgeConfig;
use std::{num::NonZeroUsize, sync::Arc};

/// Attempts to match the test name to a network benchmark test
pub fn get_netbench_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        // Network tests without chaos
        "net_bench_no_chaos_1000" => net_bench_no_chaos(MEGABYTE, 1000),
        "net_bench_no_chaos_900" => net_bench_no_chaos(MEGABYTE, 900),
        "net_bench_no_chaos_800" => net_bench_no_chaos(MEGABYTE, 800),
        "net_bench_no_chaos_700" => net_bench_no_chaos(MEGABYTE, 700),
        "net_bench_no_chaos_600" => net_bench_no_chaos(MEGABYTE, 600),
        "net_bench_no_chaos_500" => net_bench_no_chaos(MEGABYTE, 500),
        "net_bench_no_chaos_300" => net_bench_no_chaos(MEGABYTE, 300),
        "net_bench_no_chaos_200" => net_bench_no_chaos(MEGABYTE, 200),
        "net_bench_no_chaos_100" => net_bench_no_chaos(MEGABYTE, 100),
        "net_bench_no_chaos_50" => net_bench_no_chaos(MEGABYTE, 50),
        "net_bench_no_chaos_20" => net_bench_no_chaos(MEGABYTE, 20),
        "net_bench_no_chaos_10" => net_bench_no_chaos(MEGABYTE, 10),
        "net_bench_no_chaos_1" => net_bench_no_chaos(MEGABYTE, 1),

        // Network tests with chaos
        "net_bench_two_region_chaos_1000" => net_bench_two_region_chaos(MEGABYTE, 1000),
        "net_bench_two_region_chaos_500" => net_bench_two_region_chaos(MEGABYTE, 500),
        "net_bench_two_region_chaos_300" => net_bench_two_region_chaos(MEGABYTE, 300),
        "net_bench_two_region_chaos_200" => net_bench_two_region_chaos(MEGABYTE, 200),
        "net_bench_two_region_chaos_100" => net_bench_two_region_chaos(MEGABYTE, 100),
        "net_bench_two_region_chaos_50" => net_bench_two_region_chaos(MEGABYTE, 50),
        "net_bench_two_region_chaos_30" => net_bench_two_region_chaos(MEGABYTE, 30),
        "net_bench_two_region_chaos_20" => net_bench_two_region_chaos(MEGABYTE, 20),
        "net_bench_two_region_chaos_15" => net_bench_two_region_chaos(MEGABYTE, 15),
        "net_bench_two_region_chaos_10" => net_bench_two_region_chaos(MEGABYTE, 10),
        "net_bench_two_region_chaos_1" => net_bench_two_region_chaos(MEGABYTE, 1),

        // Network tests with small messages
        "net_bench_two_region_chaos_small_messages_5" => {
            net_bench_two_region_chaos(100 * KILOBYTE, 50)
        },
        "net_bench_two_region_chaos_small_messages_1" => {
            net_bench_two_region_chaos(100 * KILOBYTE, 10)
        },

        _ => return None, // The test name does not match a network benchmark test
    };
    Some(test)
}

/// Creates a netbench configuration for direct send using
/// the specified message size and frequency.
pub(crate) fn create_direct_send_netbench_config(
    message_size: usize,
    message_frequency: u64,
) -> NetbenchConfig {
    // Create the netbench config
    let mut netbench_config = NetbenchConfig::default();

    // Enable direct send network benchmarking
    netbench_config.enabled = true;
    netbench_config.enable_direct_send_testing = true;

    // Configure the message sizes and frequency
    netbench_config.direct_send_data_size = message_size;
    netbench_config.direct_send_per_second = message_frequency;
    netbench_config.max_network_channel_size = message_frequency * 2; // Double the channel size for an additional buffer

    netbench_config
}

/// Performs direct send network benchmarking between 2 validators
/// using the specified message size and frequency.
pub(crate) fn net_bench_no_chaos(message_size: usize, message_frequency: u64) -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(Delay::new(180))
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            let netbench_config =
                create_direct_send_netbench_config(message_size, message_frequency);
            config.netbench = Some(netbench_config);
        }))
}

/// Performs direct send network benchmarking between 2 validators
/// using the specified message size and frequency, with two-region chaos.
pub(crate) fn net_bench_two_region_chaos(
    message_size: usize,
    message_frequency: u64,
) -> ForgeConfig {
    net_bench_two_region_inner(create_direct_send_netbench_config(
        message_size,
        message_frequency,
    ))
}

/// A simple utility function for creating a ForgeConfig with a
/// two-region environment using the specified netbench config.
pub(crate) fn net_bench_two_region_inner(netbench_config: NetbenchConfig) -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(wrap_with_two_region_env(Delay::new(180)))
        .with_initial_validator_count(NonZeroUsize::new(2).unwrap())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.netbench = Some(netbench_config);
        }))
}
