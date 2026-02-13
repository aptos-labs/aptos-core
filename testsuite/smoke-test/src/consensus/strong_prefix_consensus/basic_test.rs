// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Basic smoke tests for Strong Prefix Consensus protocol
//!
//! These tests spawn a local swarm of validators, trigger strong prefix consensus
//! via configuration, and verify the protocol completes successfully with correct outputs.

use super::helpers::{
    cleanup_all_output_files, cleanup_output_files, generate_test_hashes,
    wait_for_strong_pc_outputs,
};
use crate::smoke_test_environment::SwarmBuilder;
use aptos_crypto::HashValue;
use std::{sync::Arc, time::Duration};

/// Test strong prefix consensus with all validators having identical inputs
///
/// This is the simplest case: all validators propose the same vector,
/// so the maximum common prefix (v_low) and minimum common extension (v_high)
/// should both equal the input. With identical inputs the protocol should
/// complete in View 1 (optimistic path).
#[tokio::test]
async fn test_strong_prefix_consensus_identical_inputs() {
    // Generate test input: 5 deterministic hashes
    let test_input = generate_test_hashes(5);
    let test_input_hex: Vec<String> = test_input.iter().map(|h| h.to_hex()).collect();

    println!(
        "Test input ({} hashes): {:?}",
        test_input.len(),
        test_input_hex
    );

    // Clean up old output files BEFORE building swarm — the protocol may complete during startup
    cleanup_all_output_files();

    // Build swarm with 4 validators, all configured with same input
    let swarm = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(move |_, config, _| {
            config.consensus.strong_prefix_consensus_test_input =
                Some(test_input_hex.clone());
        }))
        .with_aptos()
        .build()
        .await;

    // Get validator peer IDs
    let peer_ids: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    println!("Validator peer IDs: {:?}", peer_ids);

    println!("Swarm launched successfully");

    // Wait for all validators to complete and write output files
    // Strong PC may need multiple views; using 60s timeout for safety
    println!("Waiting for strong prefix consensus to complete...");
    let outputs = wait_for_strong_pc_outputs(&peer_ids, Duration::from_secs(60))
        .await
        .expect("Failed to get strong prefix consensus outputs");

    println!("\n=== Strong Prefix Consensus Outputs ===");
    for (i, output) in outputs.iter().enumerate() {
        println!("\nValidator {} (peer_id: {}):", i, output.party_id);
        println!("  Epoch: {}", output.epoch);
        println!("  Slot: {}", output.slot);
        println!("  Input: {:?}", output.input);
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

        // Property 1: Upper Bound (v_low is a prefix of v_high)
        assert!(
            v_low.len() <= v_high.len(),
            "Validator {}: v_low length ({}) should be <= v_high length ({})",
            i,
            v_low.len(),
            v_high.len()
        );

        for (j, (low_hash, high_hash)) in v_low.iter().zip(v_high.iter()).enumerate() {
            assert_eq!(
                low_hash, high_hash,
                "Validator {}: v_low[{}] != v_high[{}]",
                i, j, j
            );
        }

        println!("  Validator {}: Upper bound property satisfied", i);

        // Property 2: Validity — for identical inputs, v_low should equal the full input
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

        println!("  Validator {}: Validity property satisfied (v_low = input)", i);

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

        println!(
            "  Validator {}: v_high equals input (expected for identical inputs)",
            i
        );
    }

    // Verify Agreement: all validators have the same v_high
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

    println!("\n  All validators have consistent outputs (Agreement satisfied)");
    println!("\n=== Test PASSED ===");

    // Cleanup
    cleanup_output_files(&peer_ids);
}

/// Test strong prefix consensus with validators having partially overlapping inputs
///
/// All validators share entries at positions 0, 1, and 3, but have different
/// entries at position 2. This tests the protocol's ability to compute the
/// maximum common prefix (mcp) when inputs diverge.
///
/// Expected behavior:
/// - v_low should be [hash1, hash2] (mcp = first 2 entries)
/// - v_high should extend v_low with valid entries
/// - All validators agree on v_high (Agreement)
#[tokio::test]
async fn test_strong_prefix_consensus_divergent_inputs() {
    // Generate common hashes for positions 0, 1, 3
    let hash1 = HashValue::sha3_256_of(&(1u64).to_le_bytes());
    let hash2 = HashValue::sha3_256_of(&(2u64).to_le_bytes());
    let hash4 = HashValue::sha3_256_of(&(4u64).to_le_bytes());

    // Generate different hashes for position 2 (one per validator)
    let hash3_variants: Vec<HashValue> = (10..14)
        .map(|i| HashValue::sha3_256_of(&(i as u64).to_le_bytes()))
        .collect();

    // Build input vectors for each validator: [hash1, hash2, hash3_variant, hash4]
    let validator_inputs: Vec<Vec<String>> = hash3_variants
        .iter()
        .map(|hash3| {
            vec![
                hash1.to_hex(),
                hash2.to_hex(),
                hash3.to_hex(),
                hash4.to_hex(),
            ]
        })
        .collect();

    println!("Validator inputs:");
    for (i, input) in validator_inputs.iter().enumerate() {
        println!("  Validator {}: {:?}", i, input);
    }

    // Clean up old output files BEFORE building swarm — the protocol may complete during startup
    cleanup_all_output_files();

    // Build swarm with 4 validators, each with different input at position 2
    let swarm = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(move |idx, config, _| {
            config.consensus.strong_prefix_consensus_test_input =
                Some(validator_inputs[idx].clone());
        }))
        .with_aptos()
        .build()
        .await;

    // Get validator peer IDs
    let peer_ids: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    println!("Validator peer IDs: {:?}", peer_ids);

    println!("Swarm launched successfully");

    // Wait for all validators to complete and write output files
    // Divergent inputs may require multiple views; using 60s timeout
    println!("Waiting for strong prefix consensus to complete...");
    let outputs = wait_for_strong_pc_outputs(&peer_ids, Duration::from_secs(60))
        .await
        .expect("Failed to get strong prefix consensus outputs");

    println!("\n=== Strong Prefix Consensus Outputs ===");
    for (i, output) in outputs.iter().enumerate() {
        println!("\nValidator {} (peer_id: {}):", i, output.party_id);
        println!("  Epoch: {}", output.epoch);
        println!("  Slot: {}", output.slot);
        println!("  Input: {:?}", output.input);
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
        let input = output.input_hashes().expect("Failed to parse input");
        let v_low = output.v_low_hashes().expect("Failed to parse v_low");
        let v_high = output.v_high_hashes().expect("Failed to parse v_high");

        println!("\n  Validator {} output parsed successfully", i);

        // Property 1: Upper Bound (v_low is a prefix of v_high)
        assert!(
            v_low.len() <= v_high.len(),
            "Validator {}: v_low length ({}) should be <= v_high length ({})",
            i,
            v_low.len(),
            v_high.len()
        );

        for (j, (low_hash, high_hash)) in v_low.iter().zip(v_high.iter()).enumerate() {
            assert_eq!(
                low_hash, high_hash,
                "Validator {}: v_low[{}] != v_high[{}]",
                i, j, j
            );
        }

        println!("  Validator {}: Upper bound property satisfied", i);

        // Property 2: Validity — v_low should be a prefix of the validator's input
        assert!(
            v_low.len() <= input.len(),
            "Validator {}: v_low length ({}) should be <= input length ({})",
            i,
            v_low.len(),
            input.len()
        );

        for (j, (low_hash, input_hash)) in v_low.iter().zip(input.iter()).enumerate() {
            assert_eq!(
                low_hash, input_hash,
                "Validator {}: v_low[{}] != input[{}]",
                i, j, j
            );
        }

        println!("  Validator {}: Validity property satisfied (v_low is prefix of input)", i);

        // Property 3: For divergent inputs at position 2, v_low should be [hash1, hash2]
        // (the maximum common prefix across all validators)
        assert_eq!(
            v_low.len(),
            2,
            "Validator {}: Expected v_low length 2 (mcp), got {}",
            i,
            v_low.len()
        );

        assert_eq!(
            v_low[0], hash1,
            "Validator {}: v_low[0] should equal hash1",
            i
        );

        assert_eq!(
            v_low[1], hash2,
            "Validator {}: v_low[1] should equal hash2",
            i
        );

        println!("  Validator {}: v_low equals expected mcp [hash1, hash2]", i);

        // Property 4: v_high should extend v_low
        assert!(
            v_high.len() >= v_low.len(),
            "Validator {}: v_high should extend v_low",
            i
        );

        println!("  Validator {}: v_high extends v_low", i);
    }

    // Verify Agreement: all validators have the same v_low and v_high
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

    println!("\n  All validators have consistent outputs (Agreement satisfied)");
    println!("  v_low = [hash1, hash2] (maximum common prefix)");
    println!("\n=== Test PASSED ===");

    // Cleanup
    cleanup_output_files(&peer_ids);
}
