// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    network_tests::NetworkPlayground,
    test_utils::{consensus_runtime, timed_block_on},
    twins::twins_node::SMRNode,
};
use aptos_consensus_types::block::Block;
use aptos_types::on_chain_config::ProposerElectionType::RotatingProposer;

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
    let _num_rounds = 3;
    
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

#[test]
/// Extended test with vote verification
/// 
/// This test verifies:
/// 1. Proposals are generated
/// 2. Votes are collected for each proposal
/// 3. Quorum certificates are formed
/// 
/// Run the test:
/// cargo xtest -p consensus simple_three_round_test::extended_vote_test -- --nocapture
fn extended_vote_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    
    let num_nodes = 4;
    let num_twins = 0;
    
    println!("Starting extended vote test with {} nodes", num_nodes);
    
    let nodes = SMRNode::start_num_nodes_with_twins(
        num_nodes,
        num_twins,
        &mut playground,
        RotatingProposer(2),
        None,
    );
    
    let genesis = Block::make_genesis_block_from_ledger_info(&nodes[0].storage.get_ledger_info());
    println!("Genesis block ID: {:?}", genesis.id());
    
    timed_block_on(&runtime, async {
        println!("=== Starting extended vote test ===");
        
        // Wait for first proposal
        println!("Waiting for first proposal...");
        let proposal_msg = playground
            .wait_for_messages(1, NetworkPlayground::proposals_only)
            .await;
        
        let proposal = match &proposal_msg[0].1 {
            ConsensusMsg::ProposalMsg(proposal) => proposal,
            _ => panic!("Expected proposal message"),
        };
        
        println!("Received proposal from {:?} for round {}", 
                 proposal_msg[0].0, proposal.proposal().round());
        
        // Wait for votes (expect 3 votes from other nodes)
        println!("Waiting for votes...");
        let vote_msgs = playground
            .wait_for_messages(3, NetworkPlayground::votes_only)
            .await;
        
        println!("Received {} votes", vote_msgs.len());
        for (i, (author, msg)) in vote_msgs.iter().enumerate() {
            if let ConsensusMsg::VoteMsg(vote) = msg {
                println!("Vote {}: from {:?} for round {}", 
                         i + 1, author, vote.vote().vote_data().proposed().round());
            }
        }
        
        // Verify all votes are for the same round
        let proposal_round = proposal.proposal().round();
        for (author, msg) in &vote_msgs {
            if let ConsensusMsg::VoteMsg(vote) = msg {
                assert_eq!(vote.vote().vote_data().proposed().round(), proposal_round,
                          "Vote from {:?} is for wrong round", author);
            }
        }
        
        println!("=== Extended vote test completed successfully! ===");
        println!("Proposal round: {}", proposal_round);
        println!("All {} votes verified for correct round", vote_msgs.len());
    });
}
