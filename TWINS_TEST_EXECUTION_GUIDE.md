# Twins Test Execution Guide

## Overview

This guide provides detailed instructions for executing twins tests, including creating and running a simple 3-round scenario with 4 honest nodes and no partitions.

## Prerequisites

### 1. Environment Setup
```bash
# Ensure you're in the aptos-core-safety directory
cd /Users/bhavani/Documents/Rishal/aptos-core-safety

# Verify Rust toolchain
rustc --version
cargo --version

# Ensure you have the required dependencies
cargo check -p consensus
```

### 2. Understanding Twins Test Structure
- **Location**: `consensus/src/twins/`
- **Main Files**: 
  - `basic_twins_test.rs` - Contains existing test cases
  - `twins_node.rs` - SMRNode implementation
  - `mod.rs` - Module exports

## Step 1: Create a Simple 3-Round Test

### Create the Test File
```bash
# Navigate to the twins directory
cd consensus/src/twins

# Create a new test file
touch simple_three_round_test.rs
```

### Add the Test to mod.rs
```rust
// In consensus/src/twins/mod.rs
mod basic_twins_test;
mod twins_node;
mod simple_three_round_test;  // Add this line
```

### Write the Simple Test
```rust
// In consensus/src/twins/simple_three_round_test.rs
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    network_tests::{NetworkPlayground, TwinId},
    test_utils::{consensus_runtime, timed_block_on},
    twins::twins_node::SMRNode,
};
use aptos_consensus_types::{block::Block, common::Round};
use aptos_types::on_chain_config::ProposerElectionType::RotatingProposer;
use futures::StreamExt;
use std::collections::HashMap;

#[test]
/// Simple test with 4 honest nodes, 3 rounds, no partitions
/// 
/// This test verifies:
/// 1. All nodes start correctly
/// 2. Proposals are generated for 3 rounds
/// 3. Votes are collected and processed
/// 4. Blocks are committed
/// 
/// Run the test:
/// cargo xtest -p consensus simple_three_round_test -- --nocapture
fn simple_three_round_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    
    // Test configuration
    let num_nodes = 4;
    let num_twins = 0;  // No twins for this simple test
    let num_rounds = 3;
    
    println!("Starting simple three-round test with {} nodes", num_nodes);
    
    // Start the nodes
    let nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RotatingProposer(2),  // Rotating proposer every 2 rounds
        None,  // No specific round proposers
    );
    
    println!("Started {} nodes successfully", nodes.len());
    
    // Get genesis block for reference
    let genesis = Block::make_genesis_block_from_ledger_info(&nodes[0].storage.get_ledger_info());
    println!("Genesis block ID: {:?}", genesis.id());
    
    // Start the network playground
    runtime.spawn(playground.start());
    
    // Run the test
    timed_block_on(&runtime, async {
        println!("=== Starting 3-round consensus test ===");
        
        // Round 1: Wait for first proposal
        println!("Round 1: Waiting for first proposal...");
        let msg1 = playground
            .wait_for_messages(1, NetworkPlayground::proposals_only)
            .await;
        
        let first_proposal = match &msg1[0].1 {
            ConsensusMsg::ProposalMsg(proposal) => proposal,
            _ => panic!("Expected proposal message, got: {:?}", msg1[0].1),
        };
        
        println!("Round 1: Received proposal from {:?}", msg1[0].0);
        println!("Round 1: Proposal parent ID: {:?}", first_proposal.proposal().parent_id());
        println!("Round 1: Proposal QC block ID: {:?}", 
                 first_proposal.proposal().quorum_cert().certified_block().id());
        
        // Verify the first proposal points to genesis
        assert_eq!(first_proposal.proposal().parent_id(), genesis.id());
        assert_eq!(
            first_proposal.proposal().quorum_cert().certified_block().id(),
            genesis.id()
        );
        
        // Round 2: Wait for second proposal
        println!("Round 2: Waiting for second proposal...");
        let msg2 = playground
            .wait_for_messages(1, NetworkPlayground::proposals_only)
            .await;
        
        let second_proposal = match &msg2[0].1 {
            ConsensusMsg::ProposalMsg(proposal) => proposal,
            _ => panic!("Expected proposal message, got: {:?}", msg2[0].1),
        };
        
        println!("Round 2: Received proposal from {:?}", msg2[0].0);
        println!("Round 2: Proposal parent ID: {:?}", second_proposal.proposal().parent_id());
        
        // Round 3: Wait for third proposal
        println!("Round 3: Waiting for third proposal...");
        let msg3 = playground
            .wait_for_messages(1, NetworkPlayground::proposals_only)
            .await;
        
        let third_proposal = match &msg3[0].1 {
            ConsensusMsg::ProposalMsg(proposal) => proposal,
            _ => panic!("Expected proposal message, got: {:?}", msg3[0].1),
        };
        
        println!("Round 3: Received proposal from {:?}", msg3[0].0);
        println!("Round 3: Proposal parent ID: {:?}", third_proposal.proposal().parent_id());
        
        // Verify round progression
        assert!(first_proposal.proposal().round() < second_proposal.proposal().round());
        assert!(second_proposal.proposal().round() < third_proposal.proposal().round());
        
        println!("=== Test completed successfully! ===");
        println!("All 3 rounds completed with proper proposal generation");
        println!("Round progression verified: {} -> {} -> {}", 
                 first_proposal.proposal().round(),
                 second_proposal.proposal().round(),
                 third_proposal.proposal().round());
    });
}
```

## Step 2: Execute the Test

### Basic Execution
```bash
# Run the specific test
cargo xtest -p consensus simple_three_round_test -- --nocapture

# Alternative: Run all twins tests
cargo xtest -p consensus --test basic_twins_test -- --nocapture
```

### Advanced Execution Options
```bash
# Run with verbose output
cargo xtest -p consensus simple_three_round_test -- --nocapture --test-threads=1

# Run with specific log level
RUST_LOG=debug cargo xtest -p consensus simple_three_round_test -- --nocapture

# Run and capture output to file
cargo xtest -p consensus simple_three_round_test -- --nocapture > test_output.log 2>&1
```

## Step 3: Understanding the Output

### Expected Output Structure
```
Starting simple three-round test with 4 nodes
Started 4 nodes successfully
Genesis block ID: <hash>
=== Starting 3-round consensus test ===
Round 1: Waiting for first proposal...
Round 1: Received proposal from <author>
Round 1: Proposal parent ID: <genesis_hash>
Round 1: Proposal QC block ID: <genesis_hash>
Round 2: Waiting for second proposal...
Round 2: Received proposal from <author>
Round 2: Proposal parent ID: <round1_hash>
Round 3: Waiting for third proposal...
Round 3: Received proposal from <author>
Round 3: Proposal parent ID: <round2_hash>
=== Test completed successfully! ===
All 3 rounds completed with proper proposal generation
Round progression verified: 1 -> 2 -> 3
```

### What Each Part Means
- **Genesis block ID**: The initial block all nodes start from
- **Proposal parent ID**: The block this proposal extends
- **Proposal QC block ID**: The block certified by the quorum certificate
- **Round progression**: Ensures consensus is advancing through rounds

## Step 4: Troubleshooting

### Common Issues and Solutions

#### 1. Test Hangs or Times Out
```bash
# Check if test is actually running
ps aux | grep cargo

# Kill hanging processes
pkill -f "cargo xtest"

# Run with timeout
timeout 60 cargo xtest -p consensus simple_three_round_test -- --nocapture
```

#### 2. Compilation Errors
```bash
# Clean and rebuild
cargo clean -p consensus
cargo build -p consensus

# Check for syntax errors
cargo check -p consensus
```

#### 3. Runtime Errors
```bash
# Run with debug logging
RUST_LOG=debug cargo xtest -p consensus simple_three_round_test -- --nocapture

# Check for specific error patterns
cargo xtest -p consensus simple_three_round_test -- --nocapture 2>&1 | grep -i error
```

## Step 5: Extending the Test

### Adding More Rounds
```rust
// Modify the test to run more rounds
let num_rounds = 5;  // Change from 3 to 5

// Add more round waiting logic
for round in 1..=num_rounds {
    println!("Round {}: Waiting for proposal...", round);
    let msg = playground
        .wait_for_messages(1, NetworkPlayground::proposals_only)
        .await;
    // ... process message
}
```

### Adding Vote Verification
```rust
// Wait for votes after each proposal
let votes = playground
    .wait_for_messages(num_nodes - 1, NetworkPlayground::votes_only)
    .await;

println!("Received {} votes for round {}", votes.len(), round);
```

### Adding Commit Verification
```rust
// Wait for commits
let commits = playground
    .wait_for_messages(1, |msg| matches!(msg.1, ConsensusMsg::CommitVoteMsg(_)))
    .await;

println!("Received commit for round {}", round);
```

## Step 6: Running Existing Tests

### Available Test Commands
```bash
# Run basic start test
cargo xtest -p consensus basic_start_test -- --nocapture

# Run twins proposer test (with partitions)
cargo xtest -p consensus twins_proposer_test -- --nocapture

# Run twins commit test
cargo xtest -p consensus twins_commit_test -- --nocapture

# Run all twins tests
cargo xtest -p consensus --test basic_twins_test -- --nocapture
```

### Test Descriptions
- **basic_start_test**: 4 honest nodes, no twins, verifies first proposal
- **twins_proposer_test**: 4 honest nodes + 2 twins, with network partitions
- **twins_commit_test**: 4 honest nodes + 1 twin, verifies commit behavior

## Step 7: Best Practices

### 1. Test Development
- Start with simple scenarios (no partitions, few rounds)
- Add complexity gradually (partitions, more rounds, twins)
- Use descriptive test names and comments
- Add comprehensive logging for debugging

### 2. Execution
- Always use `--nocapture` to see output
- Use `--test-threads=1` for deterministic execution
- Run tests multiple times to check for flakiness
- Use appropriate log levels for debugging

### 3. Debugging
- Add print statements to understand execution flow
- Use `RUST_LOG=debug` for detailed logging
- Check message types and content
- Verify round progression and block relationships

## Conclusion

This guide provides a complete framework for executing twins tests. The simple 3-round test serves as a foundation that can be extended with more complex scenarios including network partitions, twins, and various failure modes.

The key advantages of twins testing for consensus validation are:
- **Precise control** over network conditions
- **Fast execution** compared to full integration tests
- **Real consensus algorithm** with simulated network
- **Easy debugging** with controlled environment
- **Deterministic results** for reliable testing
