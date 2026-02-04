// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Basic smoke tests for Prefix Consensus protocol
//!
//! These tests spawn a local swarm of validators, trigger prefix consensus via
//! configuration, and verify the protocol completes successfully with correct outputs.

use crate::smoke_test_environment::SwarmBuilder;
use super::helpers::{cleanup_output_files, generate_test_hashes, wait_for_prefix_consensus_outputs};
use std::{sync::Arc, time::Duration};

/// Test prefix consensus with all validators having identical inputs
///
/// This is the simplest case: all validators propose the same vector,
/// so the maximum common prefix (v_low) and minimum common extension (v_high)
/// should both equal the input.
#[tokio::test]
async fn test_prefix_consensus_identical_inputs() {
    // Generate test input: 3 deterministic hashes
    let test_input = generate_test_hashes(3);
    let test_input_hex: Vec<String> = test_input.iter().map(|h| h.to_hex()).collect();

    println!("Test input ({} hashes): {:?}", test_input.len(), test_input_hex);

    // Build swarm with 4 validators, all configured with same input
    let mut swarm = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(move |_, config, _| {
            // Configure prefix consensus test input
            config.consensus.prefix_consensus_test_input = Some(test_input_hex.clone());
        }))
        .with_aptos()
        .build()
        .await;

    // Get validator peer IDs
    let peer_ids: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    println!("Validator peer IDs: {:?}", peer_ids);

    // Clean up any old output files
    cleanup_output_files(&peer_ids);

    // Swarm is already launched by build(). Prefix consensus will start on epoch startup.
    println!("Swarm launched successfully");

    // Wait for all validators to complete and write output files
    // Protocol should complete in ~5-10 seconds, using 30s timeout for safety
    println!("Waiting for prefix consensus to complete...");
    let outputs = wait_for_prefix_consensus_outputs(&peer_ids, Duration::from_secs(30))
        .await
        .expect("Failed to get prefix consensus outputs");

    println!("\n=== Prefix Consensus Outputs ===");
    for (i, output) in outputs.iter().enumerate() {
        println!("\nValidator {} (peer_id: {}):", i, output.party_id);
        println!("  Epoch: {}", output.epoch);
        println!("  v_low: {:?}", output.v_low);
        println!("  v_high: {:?}", output.v_high);
    }

    // Verify all validators completed
    assert_eq!(
        outputs.len(),
        4,
        "Expected 4 validator outputs, got {}",
        outputs.len()
    );

    // Verify correctness properties
    for (i, output) in outputs.iter().enumerate() {
        let v_low = output.v_low_hashes().expect("Failed to parse v_low");
        let v_high = output.v_high_hashes().expect("Failed to parse v_high");

        // Property 1: Upper Bound (v_low ⪯ v_high)
        assert!(
            v_low.len() <= v_high.len(),
            "Validator {}: v_low length ({}) should be <= v_high length ({})",
            i,
            v_low.len(),
            v_high.len()
        );

        // v_low should be a prefix of v_high
        for (j, (low_hash, high_hash)) in v_low.iter().zip(v_high.iter()).enumerate() {
            assert_eq!(
                low_hash, high_hash,
                "Validator {}: v_low[{}] != v_high[{}]",
                i, j, j
            );
        }

        println!("✓ Validator {}: Upper bound property satisfied", i);

        // Property 2: Validity (for identical inputs, mcp = input, so v_low = input)
        assert_eq!(
            v_low.len(),
            test_input.len(),
            "Validator {}: v_low length ({}) should equal input length ({})",
            i,
            v_low.len(),
            test_input.len()
        );

        for (j, (output_hash, input_hash)) in v_low.iter().zip(test_input.iter()).enumerate() {
            assert_eq!(
                output_hash, input_hash,
                "Validator {}: v_low[{}] != input[{}]",
                i, j, j
            );
        }

        println!("✓ Validator {}: Validity property satisfied", i);

        // Property 3: For identical inputs, v_high should also equal input
        assert_eq!(
            v_high.len(),
            test_input.len(),
            "Validator {}: v_high length ({}) should equal input length ({})",
            i,
            v_high.len(),
            test_input.len()
        );

        for (j, (output_hash, input_hash)) in v_high.iter().zip(test_input.iter()).enumerate() {
            assert_eq!(
                output_hash, input_hash,
                "Validator {}: v_high[{}] != input[{}]",
                i, j, j
            );
        }

        println!("✓ Validator {}: v_high equals input (expected for identical inputs)", i);
    }

    // Verify consistency across validators (all should have same output)
    let first_v_low = outputs[0].v_low.clone();
    let first_v_high = outputs[0].v_high.clone();

    for (i, output) in outputs.iter().enumerate().skip(1) {
        assert_eq!(
            output.v_low, first_v_low,
            "Validator {} v_low differs from validator 0",
            i
        );
        assert_eq!(
            output.v_high, first_v_high,
            "Validator {} v_high differs from validator 0",
            i
        );
    }

    println!("\n✓ All validators have consistent outputs");
    println!("\n=== Test PASSED ===");

    // Cleanup
    cleanup_output_files(&peer_ids);
}
