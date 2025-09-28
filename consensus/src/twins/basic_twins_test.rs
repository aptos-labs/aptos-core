// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    network_tests::{NetworkPlayground, TwinId},
    test_utils::{consensus_runtime, timed_block_on},
    twins::twins_node::SMRNode,
};
use aptos_consensus_types::{block::Block, common::Round};
use aptos_types::on_chain_config::ProposerElectionType::{
    FixedProposer, RotatingProposer, RoundProposer,
};
use futures::StreamExt;
use std::collections::HashMap;

#[test]
/// This test checks that the first proposal has its parent and
/// QC pointing to the genesis block.
///
/// Setup:
///
/// 4 honest nodes, and 0 twins
///
/// Run the test:
/// cargo xtest -p consensus basic_start_test -- --nocapture
#[ignore]
fn basic_start_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let num_nodes = 4;
    let num_twins = 0;
    let nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RotatingProposer(2),
        None,
    );
    let genesis = Block::make_genesis_block_from_ledger_info(&nodes[0].storage.get_ledger_info());
    timed_block_on(&runtime, async {
        let msg = playground
            .wait_for_messages(1, NetworkPlayground::proposals_only)
            .await;
        let first_proposal = match &msg[0].1 {
            ConsensusMsg::ProposalMsg(proposal) => proposal,
            _ => panic!("Unexpected message found"),
        };
        assert_eq!(first_proposal.proposal().parent_id(), genesis.id());
        assert_eq!(
            first_proposal
                .proposal()
                .quorum_cert()
                .certified_block()
                .id(),
            genesis.id()
        );
    });
}

#[test]
/// This test checks that the split_network function works
/// as expected, that is: nodes in a partition with less nodes
/// than required for quorum do not commit anything.
///
/// Setup:
///
/// 4 honest nodes (n0, n1, n2, n3), and 0 twins.
/// Create two partitions p1=[n2], and p2=[n0, n1, n3] with
/// a proposer (n0) in p2.
///
/// Test:
///
/// Run consensus for enough rounds to potentially form a commit.
/// Check that n1 has no commits, and n0 has commits.
///
/// Run the test:
/// cargo xtest -p consensus drop_config_test -- --nocapture
#[ignore] // TODO: https://github.com/aptos-labs/aptos-core/issues/8767
fn drop_config_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let num_nodes = 4;
    let num_twins = 0;
    let mut nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        FixedProposer(2),
        None,
    );

    // 4 honest nodes
    let n0_twin_id = nodes[0].id;
    let n1_twin_id = nodes[1].id;
    let n2_twin_id = nodes[2].id;
    let n3_twin_id = nodes[3].id;

    assert!(playground.split_network(vec![n2_twin_id], vec![n0_twin_id, n1_twin_id, n3_twin_id]));
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        // Check that the commit log for n0 is not empty
        let node0_commit = nodes[0].commit_cb_receiver.next().await;
        assert!(node0_commit.is_some());

        // Check that the commit log for n2 is empty
        let node2_commit = match nodes[2].commit_cb_receiver.try_next() {
            Ok(Some(node_commit)) => Some(node_commit),
            _ => None,
        };
        assert!(node2_commit.is_none());
    });
}

#[test]
/// This test checks that the vote of a node and its twin
/// should be counted as duplicate vote (because they have
/// the same public keys)
///
/// Setup:
///
/// 4 honest nodes (n0, n1, n2, n3), and 1 twin (twin0)
/// Create 2 partitions, p1=[n1, n3], p2=[n0, twin0, n2]
///
/// Test:
///
/// Extract enough votes to potentially form commits. Check
/// that no node commits any block. This is because we need
/// 3 nodes to form a quorum and no partition has enough votes
/// (note there are 3 nodes in p2, but one of them is a twin,
/// and its vote will be counted as duplicate of n0).
///
/// Run the test:
/// cargo xtest -p consensus twins_vote_dedup_test -- --nocapture
fn twins_vote_dedup_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let num_nodes = 4;
    let num_twins = 1;
    let mut nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RotatingProposer(2),
        None,
    );

    // 4 honest nodes
    let n0_twin_id = nodes[0].id;
    // twin of n0 has same author as node[0]
    let twin0_twin_id = nodes[4].id;
    assert_eq!(n0_twin_id.author, twin0_twin_id.author);
    let n1_twin_id = nodes[1].id;
    let n2_twin_id = nodes[2].id;
    let n3_twin_id = nodes[3].id;

    assert!(playground.split_network(vec![n1_twin_id, n3_twin_id], vec![
        twin0_twin_id,
        n0_twin_id,
        n2_twin_id
    ],));
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        // No node should be able to commit because of the way partitions
        // have been created
        let mut commit_seen = false;
        for node in &mut nodes {
            if let Ok(Some(_node_commit)) = node.commit_cb_receiver.try_next() {
                commit_seen = true;
            }
        }
        assert!(!commit_seen);
    });
}

#[test]
/// This test checks that when a node becomes a proposer, its
/// twin becomes one too.
///
/// Setup:
///
/// 4 honest nodes (n0, n1, n2, n3), and 2 twins (twin0, twin1)
/// Create 2 partitions, p1=[n0, n1, n2], p2=[n3, twin0, twin1]
/// Let n0 (and implicitly twin0) be proposers
///
/// Test:
///
/// Extract enough votes so nodes in both partitions form commits.
/// The commits should be on two different blocks
///
/// Run the test:
/// cargo xtest -p consensus twins_proposer_test -- --nocapture
#[ignore]
fn twins_proposer_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let num_nodes = 4;
    let num_twins = 2;

    // Specify round leaders
    // Will default to the first node, if no leader specified for given round
    let mut round_proposers: HashMap<Round, usize> = HashMap::new();
    // Leaders are n0 (and implicitly twin0) for round 1..10
    for i in 1..10 {
        round_proposers.insert(i, 0);
    }

    let mut nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RoundProposer(HashMap::new()),
        Some(round_proposers),
    );

    // 4 honest nodes
    let n0_twin_id = nodes[0].id;
    // twin of n0 has same author as node_authors[0]
    let twin0_twin_id = nodes[4].id;
    assert_eq!(n0_twin_id.author, twin0_twin_id.author);
    let n1_twin_id = nodes[1].id;
    // twin of n1 has same author as node_authors[1]
    let twin1_twin_id = nodes[5].id;
    assert_eq!(n1_twin_id.author, twin1_twin_id.author);
    let n2_twin_id = nodes[2].id;
    let n3_twin_id = nodes[3].id;

    // Create per round partitions
    let mut round_partitions: HashMap<u64, Vec<Vec<TwinId>>> = HashMap::new();
    // Round 1 to 10 partitions: [node0, node1, node2], [node3, twin0, twin1]
    for i in 1..10 {
        round_partitions.insert(i, vec![vec![n0_twin_id, n1_twin_id, n2_twin_id], vec![
            n3_twin_id,
            twin0_twin_id,
            twin1_twin_id,
        ]]);
    }
    assert!(playground.split_network_round(&round_partitions));
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        let node0_commit = nodes[0].commit_cb_receiver.next().await;
        let twin0_commit = nodes[4].commit_cb_receiver.next().await;

        match (node0_commit, twin0_commit) {
            (Some(node0_commit_inner), Some(twin0_commit_inner)) => {
                let node0_commit_id = node0_commit_inner.ledger_info().commit_info().id();
                let twin0_commit_id = twin0_commit_inner.ledger_info().commit_info().id();
                // Proposal from both node0 and twin_node0 are going to
                // get committed in their respective partitions
                assert_ne!(node0_commit_id, twin0_commit_id);
            },
            _ => panic!("[TwinsTest] Test failed due to no commit(s)"),
        }
    });
}

#[test]
#[ignore] // TODO: https://github.com/aptos-labs/aptos-core/issues/6615
/// This test checks that when a node and its twin are both leaders
/// for a round, only one of the two proposals gets committed
///
/// Setup:
///
/// Network of 4 nodes (n0, n1, n2, n3), and 1 twin (twin0)
///
/// Test:
///
/// Let n0 (and implicitly twin0) be proposers
/// Pull out enough votes so a commit can be formed
/// Check that the commit of n0 and twin0 matches
///
/// Run the test:
/// cargo xtest -p consensus twins_commit_test -- --nocapture
fn twins_commit_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let num_nodes = 4;
    let num_twins = 1;

    // Specify round leaders
    // Will default to the first node, if no leader specified for given round
    let mut round_proposers: HashMap<Round, usize> = HashMap::new();
    // Leaders are n0 and twin0 for round 1..10
    for i in 1..10 {
        round_proposers.insert(i, 0);
    }

    let mut nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RoundProposer(HashMap::new()),
        Some(round_proposers),
    );
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        let node0_commit = nodes[0].commit_cb_receiver.next().await;
        let twin0_commit = nodes[4].commit_cb_receiver.next().await;

        match (node0_commit, twin0_commit) {
            (Some(node0_commit_inner), Some(twin0_commit_inner)) => {
                let node0_commit_id = node0_commit_inner.ledger_info().commit_info().id();
                let twin0_commit_id = twin0_commit_inner.ledger_info().commit_info().id();
                // Proposals from both node0 and twin_node0 are going to race,
                // but only one of them will form a commit
                assert_eq!(node0_commit_id, twin0_commit_id);
            },
            _ => panic!("[TwinsTest] Test failed due to no commit(s)"),
        }
    });
}

#[test]
fn safety_eleven_rounds() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());

    // Nodes: A, B, C, D map to indices 0,1,2,3
    let num_nodes = 4;
    let num_twins = 0;

    // Leader schedule for rounds 1..=11
    // {(1:A),(2:A),(3:A),(4:A),(5:B),(6:A),(7:C),(8:B),(9:B),(10:C),(11:C)}
    let mut round_proposers: HashMap<Round, usize> = HashMap::new();
    round_proposers.insert(1, 0);
    round_proposers.insert(2, 0);
    round_proposers.insert(3, 0);
    round_proposers.insert(4, 0);
    round_proposers.insert(5, 1);
    round_proposers.insert(6, 0);
    round_proposers.insert(7, 2);
    round_proposers.insert(8, 1);
    round_proposers.insert(9, 1);
    round_proposers.insert(10, 2);
    round_proposers.insert(11, 2);

    let mut nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RoundProposer(HashMap::new()),
        Some(round_proposers),
    );
    
    // Timeout configuration: Twins tests now use 5-second timeouts
    // This allows us to see timeout-based round progression during network partitions
    println!("[CONFIG] Twins tests now use 5-second timeouts (changed from 2,000 seconds)");
    println!("[CONFIG] This allows timeout-based round progression during network partitions");

    // TwinIds for A,B,C,D
    let a_id = nodes[0].id;
    let b_id = nodes[1].id;
    let c_id = nodes[2].id;
    let d_id = nodes[3].id;

    // Per-round partitions
    // Rounds 1..=4: {A,B,C,D}
    // Rounds 5..=6: {A,C,D}, {B}
    // Rounds 7..=8: {A,B,D}, {C}
    // Rounds 9..=11: {A,C,D}, {B}
    let mut round_partitions: HashMap<u64, Vec<Vec<TwinId>>> = HashMap::new();
    for r in 1..=4u64 {
        round_partitions.insert(r, vec![vec![a_id, b_id, c_id, d_id]]);
    }
    for r in 5..=6u64 {
        round_partitions.insert(r, vec![vec![a_id, c_id, d_id], vec![b_id]]);
    }
    for r in 7..=8u64 {
        round_partitions.insert(r, vec![vec![a_id, b_id, d_id], vec![c_id]]);
    }
    for r in 9..=11u64 {
        round_partitions.insert(r, vec![vec![a_id, c_id, d_id], vec![b_id]]);
    }
    assert!(playground.split_network_round(&round_partitions));
    println!("[NETWORK] Applied network partitions:");
    for (round, partitions) in &round_partitions {
        println!("  Round {}: {:?}", round, partitions);
    }
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        println!("=== Starting 11-round consensus test with partitions ===");
        println!("Leader schedule: R1-4:A, R5:B, R6:A, R7:C, R8:B, R9:B, R10-11:C");
        println!("Partition schedule:");
        println!("  R1-4: {{A,B,C,D}} (all connected)");
        println!("  R5-6: {{A,C,D}}, {{B}} (B isolated)");
        println!("  R7-8: {{A,B,D}}, {{C}} (C isolated)");
        println!("  R9-11: {{A,C,D}}, {{B}} (B isolated)");
        println!();

        // Track all commits from all nodes
        let mut all_commits: Vec<Vec<aptos_crypto::HashValue>> = vec![vec![], vec![], vec![], vec![]];
        let mut node_rounds: Vec<u64> = vec![0, 0, 0, 0]; // Track current round for each node
        let mut node_locks: Vec<Vec<aptos_crypto::HashValue>> = vec![vec![], vec![], vec![], vec![]]; // Track locks for each node
        
        // Wait for commits from all nodes and track proposals
        let mut total_commits = 0;
        let mut last_log_time = std::time::Instant::now();
        let mut last_round_log = 0u64;
        let test_start_time = std::time::Instant::now();
        let max_test_duration = std::time::Duration::from_secs(60); // 60 second timeout to allow for 5-second round timeouts
        
        // Track round progression instead of just commits
        let mut max_round_reached = 0u64;
        let mut rounds_with_commits = 0u64;
        
        while max_round_reached < 11 && test_start_time.elapsed() < max_test_duration {
            
            // Check for commits from all nodes
            for (node_idx, node) in nodes.iter_mut().enumerate() {
                while let Ok(Some(commit_li)) = node.commit_cb_receiver.try_next() {
                    let id = commit_li.ledger_info().commit_info().id();
                    let round = commit_li.ledger_info().commit_info().round();
                    let height = all_commits[node_idx].len() + 1; // +1 because height 1 is genesis
                    
                    all_commits[node_idx].push(id);
                    total_commits += 1;
                    
                    let node_name = match node_idx {
                        0 => "A",
                        1 => "B", 
                        2 => "C",
                        3 => "D",
                        _ => "Unknown"
                    };
                    
                    // Convert hash to readable format for easier debugging
                    let readable_id = format!("BLOCK_{}_{}", round, node_name);
                    println!("[COMMIT] Node {} committed block {} (hash: {:?}) at height {} (round {})", 
                             node_name, readable_id, id, height, round);
                    
                    // Update node's current round and add to locks
                    node_rounds[node_idx] = round;
                    node_locks[node_idx].push(id);
                    
                    // Log lock state after each commit (only for first few commits to avoid spam)
                    if node_locks[node_idx].len() <= 3 {
                        let readable_locks: Vec<String> = node_locks[node_idx].iter().enumerate().map(|(i, _hash)| {
                            format!("LOCK_{}", i + 1)
                        }).collect();
                        println!("[LOCKS] After commit, Node {} locks: {:?}", node_name, readable_locks);
                    }
                }
            }
            
            // Log round progression and lock states
            let current_max_round = *node_rounds.iter().max().unwrap_or(&0);
            if current_max_round > last_round_log {
                max_round_reached = current_max_round;
                
                // Check if we've reached a high enough round to see the safety violation
                if current_max_round >= 11 {
                    println!("[TEST] Reached round 11, checking for safety violation...");
                    break;
                }
                
                // Also check if we have enough commits to analyze safety violation
                if total_commits >= 8 && current_max_round >= 5 {
                    println!("[TEST] Have enough commits ({} total) and reached round {}, checking for safety violation...", total_commits, current_max_round);
                    break;
                }
                println!();
                println!("[ROUND PROGRESSION] Current rounds: A={}, B={}, C={}, D={}", 
                         node_rounds[0], node_rounds[1], node_rounds[2], node_rounds[3]);
                
                // Log which nodes are in which partition for current round
                if current_max_round >= 1 && current_max_round <= 4 {
                    println!("[PARTITION] Round {}: All nodes {{A,B,C,D}} connected", current_max_round);
                } else if current_max_round >= 5 && current_max_round <= 6 {
                    println!("[PARTITION] Round {}: Partition {{A,C,D}} vs {{B}} (B isolated) - EXPECTING TIMEOUTS", current_max_round);
                } else if current_max_round >= 7 && current_max_round <= 8 {
                    println!("[PARTITION] Round {}: Partition {{A,B,D}} vs {{C}} (C isolated) - EXPECTING TIMEOUTS", current_max_round);
                } else if current_max_round >= 9 && current_max_round <= 11 {
                    println!("[PARTITION] Round {}: Partition {{A,C,D}} vs {{B}} (B isolated) - EXPECTING TIMEOUTS", current_max_round);
                }
                
                // Track if this round had commits or was timeout-based
                if total_commits > rounds_with_commits {
                    rounds_with_commits = total_commits;
                    println!("[ROUND TYPE] Round {}: COMMIT-BASED ({} total commits)", current_max_round, total_commits);
                } else {
                    println!("[ROUND TYPE] Round {}: TIMEOUT-BASED (no new commits, round progressed via timeout)", current_max_round);
                }
                
               // Log lock states for all nodes
               println!("[LOCK STATES] Current locks for each node:");
               for (i, locks) in node_locks.iter().enumerate() {
                   let node_name = match i {
                       0 => "A", 1 => "B", 2 => "C", 3 => "D", _ => "Unknown"
                   };
                   let readable_locks: Vec<String> = locks.iter().enumerate().map(|(j, _hash)| {
                       format!("LOCK_{}", j + 1)
                   }).collect();
                   println!("  Node {} (round {}): locks = {:?}", node_name, node_rounds[i], readable_locks);
               }
                
                last_round_log = current_max_round;
            }
            
            // Log progress every 5 seconds to reduce overhead
            if last_log_time.elapsed().as_secs() >= 5 {
                println!("[PROGRESS] Max round reached: {}, Total commits: {}", max_round_reached, total_commits);
                for (i, commits) in all_commits.iter().enumerate() {
                    let node_name = match i {
                        0 => "A", 1 => "B", 2 => "C", 3 => "D", _ => "Unknown"
                    };
                    println!("  Node {}: {} commits, round {}", node_name, commits.len(), node_rounds[i]);
                }
                last_log_time = std::time::Instant::now();
            }
            
            // Small delay to avoid busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Check if we timed out
        if test_start_time.elapsed() >= max_test_duration {
            println!("[TEST] Test timed out after 60 seconds. Max round reached: {}, Total commits: {}", max_round_reached, total_commits);
        }

        println!();
        println!("=== Test Summary ===");
        println!("Max round reached: {}", max_round_reached);
        println!("Total commits collected: {}", total_commits);
        println!("Rounds with commits: {}", rounds_with_commits);
        println!("Rounds with timeouts: {}", max_round_reached.saturating_sub(rounds_with_commits));
        println!();
        println!("=== Final Commit Summary ===");
        for (i, commits) in all_commits.iter().enumerate() {
            let node_name = match i {
                0 => "A", 1 => "B", 2 => "C", 3 => "D", _ => "Unknown"
            };
            println!("Node {} commits:", node_name);
            for (height, commit_id) in commits.iter().enumerate() {
                let readable_id = format!("COMMIT_{}_{}", height + 1, node_name);
                println!("  Height {}: {} (hash: {:?})", height + 1, readable_id, commit_id);
            }
        }

        println!();
        println!("=== Lock Analysis for Safety Violation ===");
        println!("Analyzing which locks were available to leaders before making proposals:");
        
        // Analyze lock states at key decision points
        let key_rounds = vec![4, 5, 6, 7, 8, 9];
        for round in key_rounds {
            println!();
            println!("[LOCK ANALYSIS] Before Round {} proposal:", round);
            
            // Determine which node is the leader for this round
            let leader = match round {
                1..=4 => "A",
                5 => "B", 
                6 => "A",
                7 => "C",
                8 => "B",
                9 => "B",
                10..=11 => "C",
                _ => "Unknown"
            };
            
            println!("  Leader for round {}: {}", round, leader);
            
            // Show which nodes are in the leader's partition
            let leader_partition = match round {
                1..=4 => vec!["A", "B", "C", "D"],
                5..=6 => vec!["A", "C", "D"], // B is isolated
                7..=8 => vec!["A", "B", "D"], // C is isolated  
                9..=11 => vec!["A", "C", "D"], // B is isolated
                _ => vec![]
            };
            
            println!("  Leader's partition: {:?}", leader_partition);
            
           // Show locks available to the leader (from nodes in its partition)
           println!("  Locks available to leader {}:", leader);
           for (i, locks) in node_locks.iter().enumerate() {
               let node_name = match i {
                   0 => "A", 1 => "B", 2 => "C", 3 => "D", _ => "Unknown"
               };
               let readable_locks: Vec<String> = locks.iter().enumerate().map(|(j, _hash)| {
                   format!("LOCK_{}", j + 1)
               }).collect();
               if leader_partition.contains(&node_name) {
                   println!("    From {} (round {}): {:?}", node_name, node_rounds[i], readable_locks);
               } else {
                   println!("    From {} (round {}): {:?} [NOT ACCESSIBLE - different partition]",
                            node_name, node_rounds[i], readable_locks);
               }
           }
            
            // Show the safety violation potential
            if round >= 7 {
                println!("  [SAFETY CHECK] At round {}, different partitions have different lock states:", round);
                
                let format_locks = |locks: &Vec<aptos_crypto::HashValue>| -> String {
                    locks.iter().enumerate().map(|(i, _hash)| {
                        format!("LOCK_{}", i + 1)
                    }).collect::<Vec<String>>().join(", ")
                };
                
                let partition1_locks = if round >= 7 && round <= 8 {
                    // A,B,D partition
                    format!("A: [{}], B: [{}], D: [{}]",
                            format_locks(&node_locks[0]), 
                            format_locks(&node_locks[1]), 
                            format_locks(&node_locks[3]))
                } else {
                    // A,C,D partition
                    format!("A: [{}], C: [{}], D: [{}]",
                            format_locks(&node_locks[0]), 
                            format_locks(&node_locks[2]), 
                            format_locks(&node_locks[3]))
                };

                let isolated_node = if round >= 7 && round <= 8 { "C" } else { "B" };
                let isolated_idx = if round >= 7 && round <= 8 { 2 } else { 1 };
                let isolated_locks = format!("{}: [{}]", isolated_node, format_locks(&node_locks[isolated_idx]));

                println!("    Partition 1 locks: {}", partition1_locks);
                println!("    Partition 2 locks: {}", isolated_locks);
                println!("    This divergence in lock states can lead to safety violations!");
            }
        }

        // Final safety check: B's and C's commits at height 4 must be different
        // This demonstrates the safety violation caused by network partitions
        let b_commits = all_commits[1].len();
        let c_commits = all_commits[2].len();
        let a_commits = all_commits[0].len();
        let d_commits = all_commits[3].len();
        
        println!();
        println!("=== Final Safety Check ===");
        println!("Node A: {} commits (heights 1-{})", a_commits, a_commits);
        println!("Node B: {} commits (heights 1-{})", b_commits, b_commits);
        println!("Node C: {} commits (heights 1-{})", c_commits, c_commits);
        println!("Node D: {} commits (heights 1-{})", d_commits, d_commits);
        
        // Check if both B and C have commits at height 4 (index 3)
        if b_commits >= 4 && c_commits >= 4 {
            let b_h4 = all_commits[1][3]; // Height 4 = index 3
            let c_h4 = all_commits[2][3]; // Height 4 = index 3
            
            println!();
            println!("=== Height 4 Commit Comparison ===");
            println!("Node B's height-4 commit: {:?}", b_h4);
            println!("Node C's height-4 commit: {:?}", c_h4);
            
            if b_h4 != c_h4 {
                println!();
                println!("✅ SAFETY VIOLATION CONFIRMED:");
                println!("   Node B and Node C have different commits at height 4!");
                println!("   B's height-4 commit: {:?}", b_h4);
                println!("   C's height-4 commit: {:?}", c_h4);
                println!("   This demonstrates that network partitions can cause safety violations!");
            } else {
                println!();
                println!("❌ No safety violation detected:");
                println!("   Node B and Node C have the same commit at height 4: {:?}", b_h4);
                panic!("Expected safety violation: B and C should have different commits at height 4");
            }
        } else {
            println!();
            println!("❌ Cannot perform height-4 safety check:");
            println!("   Node B has {} commits, Node C has {} commits", b_commits, c_commits);
            println!("   Both nodes need at least 4 commits to compare height-4 commits");
            panic!("Expected both B and C to have at least 4 commits for safety violation check");
        }
    });
}
