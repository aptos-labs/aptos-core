# Four Node Partition Scenario - Execution Guide

## Overview

This testcase implements a specific consensus fault tolerance scenario with 4 validators:

- **Round 1 (10 seconds)**: Normal operation - all 4 nodes communicate freely
- **Round 2 (10 seconds)**: Network partition - nodes split into two groups [0,1] vs [2,3]
- **Total Duration**: ~20 seconds
- **Expected Behavior**: Test observes how consensus behaves when network is split into two equal partitions

## Test Implementation Details

### Files Created/Modified:
1. `testsuite/testcases/src/four_node_partition_scenario.rs` - Main test implementation
2. `testsuite/testcases/src/lib.rs` - Added module declaration
3. `testsuite/forge-cli/src/suites/ungrouped.rs` - Added test suite integration

### Key Features:
- **Precise Round Control**: Uses `test_consensus_fault_tolerance` framework
- **Network Partition**: Implements 100% packet loss between node groups using `SwarmNetEm`
- **Progress Monitoring**: Tracks consensus progress (epochs, rounds, transactions) per round
- **Validation**: Checks expected behavior in each round

## Prerequisites

1. **Rust Environment**: Ensure Rust is installed and configured
2. **Repository Setup**: Clone and be in the `aptos-core-safety` directory
3. **Build Dependencies**: The project should compile successfully

## Step-by-Step Execution Instructions

### Step 1: Verify Environment Setup

```bash
# Ensure you're in the correct directory
cd /path/to/aptos-core-safety

# Verify Rust is available
cargo --version

# Check if forge CLI can be built
cargo check -p aptos-forge-cli
```

### Step 2: Build the Forge CLI (if needed)

```bash
# Build the forge CLI binary
cargo build -p aptos-forge-cli --release
```

### Step 3: Execute the Test

#### Basic Execution:
```bash
cargo run -p aptos-forge-cli -- \
  --suite "four_node_partition_scenario" \
  --num-validators 4 \
  --duration_secs 30 \
  test local-swarm
```

#### Recommended Execution with Logging:
```bash
cargo run -p aptos-forge-cli -- \
  --suite "four_node_partition_scenario" \
  --num-validators 4 \
  --duration_secs 30 \
  --retain-debug-logs \
  test local-swarm --swarmdir "./four_node_partition_logs"
```

#### Advanced Execution with Verbose Output:
```bash
RUST_LOG=info cargo run -p aptos-forge-cli -- \
  --suite "four_node_partition_scenario" \
  --num-validators 4 \
  --duration_secs 30 \
  --retain-debug-logs \
  --verbose true \
  test local-swarm --swarmdir "./four_node_partition_logs_$(date +%Y%m%d_%H%M%S)"
```

### Step 4: Command Line Parameters Explained

| Parameter | Value | Description |
|-----------|--------|-------------|
| `--suite` | `"four_node_partition_scenario"` | Specifies our custom test suite |
| `--num-validators` | `4` | **Critical**: Must be exactly 4 for this test |
| `--duration_secs` | `30` | Total test duration (should be ≥20 for 2 rounds) |
| `--retain-debug-logs` | (flag) | Keeps detailed logs for all nodes |
| `--swarmdir` | `"./four_node_partition_logs"` | Directory to save all test artifacts |
| `test local-swarm` | (subcommand) | Runs test using local swarm backend |

### Step 5: Monitor Test Execution

During execution, you should see output similar to:

```
Starting 4-node partition scenario test
Round 1: Normal operation (10 seconds)
Round 2: Network partition - nodes [0,1] vs [2,3] (10 seconds)

Round 1 completed: epochs=1, rounds=5, transactions=45
Node 0: version=45, epoch=1, round=5
Node 1: version=45, epoch=1, round=5
Node 2: version=45, epoch=1, round=5
Node 3: version=45, epoch=1, round=5
Validating Round 1: Normal operation
Round progress per node: [5, 5, 5, 5]

Applying network partition: [0,1] vs [2,3]
Network partition applied successfully

Round 2 completed: epochs=1, rounds=8, transactions=67
Node 0: version=67, epoch=1, round=8
Node 1: version=67, epoch=1, round=8
Node 2: version=67, epoch=1, round=8
Node 3: version=67, epoch=1, round=8
Validating Round 2: Partition effects
Round progress per node during partition: [3, 3, 3, 3]
Partition test completed - progress: [3, 3, 3, 3]

Four-node partition scenario completed successfully!
```

## Expected Results

### Round 1 (Normal Operation):
- All 4 nodes should make consistent progress
- Transaction throughput should be normal
- All nodes should reach the same consensus state

### Round 2 (Network Partition):
- **Important**: With 4 nodes split 2-2, neither partition has a 3f+1 majority
- Progress may slow down or halt completely
- Some implementations might have one partition continue (depending on leader election)
- The test will capture the actual behavior for analysis

## Analyzing Results

### Log Analysis:

1. **Test Logs**: Check console output for round-by-round progress
2. **Node Logs**: Examine individual node logs in the swarmdir:
   ```bash
   ls -la ./four_node_partition_logs/
   # Should contain directories: 0/, 1/, 2/, 3/
   
   # Check individual node logs
   tail -f ./four_node_partition_logs/0/log
   tail -f ./four_node_partition_logs/2/log
   ```

3. **Network Behavior**: Look for evidence of partition in logs:
   - Connection timeouts between groups [0,1] and [2,3]
   - Consensus messages only within each group
   - Potential leader election changes

### Key Metrics to Observe:

- **Consensus Progress**: Rounds advanced per node per round
- **Transaction Throughput**: TPS before, during, and after partition
- **Network Behavior**: Evidence of partition isolation
- **Recovery**: How quickly consensus resumes (if it does)

## Troubleshooting

### Common Issues:

1. **Build Errors**:
   ```bash
   # Clean and rebuild
   cargo clean
   cargo build -p aptos-forge-cli
   ```

2. **Wrong Number of Validators**:
   - Error: "This test requires exactly 4 validators"
   - Solution: Ensure `--num-validators 4` is specified

3. **Permission Issues**:
   ```bash
   # Ensure swarmdir is writable
   mkdir -p ./four_node_partition_logs
   chmod 755 ./four_node_partition_logs
   ```

4. **Port Conflicts**:
   - If you get port binding errors, ensure no other Aptos nodes are running
   - Kill any existing processes: `pkill aptos-node`

### Debug Mode:

For maximum debugging information:

```bash
RUST_LOG=debug,aptos_forge=info cargo run -p aptos-forge-cli -- \
  --suite "four_node_partition_scenario" \
  --num-validators 4 \
  --duration_secs 30 \
  --retain-debug-logs \
  --verbose true \
  test local-swarm --swarmdir "./debug_four_node_partition"
```

## Understanding the Test Code

### Core Components:

1. **FourNodePartitionScenario**: Main test struct implementing `NetworkTest`
2. **FourNodeScenarioFailureInjection**: Custom failure injection implementing round-specific network partitions
3. **Network Partition Logic**: Uses `SwarmNetEm` with 100% packet loss between groups

### Customization Options:

You can modify the test by editing `testsuite/testcases/src/four_node_partition_scenario.rs`:

- **Round Duration**: Change `round_duration_secs` in the `Default` implementation
- **Partition Groups**: Modify the node indices in `apply_partition()`
- **Network Effects**: Adjust packet loss percentage or add delays instead of full partition
- **Success Criteria**: Modify expected progress validation in the main test function

## Expected Learning Outcomes

This test demonstrates:

1. **Consensus Behavior Under Partition**: How Aptos consensus handles network splits
2. **Fault Tolerance Limits**: 2-2 partition challenges for BFT consensus (needs 3f+1)
3. **Recovery Mechanisms**: How the system behaves when partition is removed
4. **Monitoring Capabilities**: Tools available for observing consensus state

## Next Steps

After running this test successfully, you can:

1. **Modify Partition Sizes**: Try 3-1 partition (should maintain consensus)
2. **Add More Rounds**: Extend the scenario with recovery rounds
3. **Test Different Network Conditions**: Use delays instead of complete partition
4. **Integrate with CI/CD**: Add as part of automated testing suite

## Support

If you encounter issues:

1. Check the troubleshooting section above
2. Review the generated logs in the swarmdir
3. Ensure all prerequisites are met
4. Verify the test implementation matches the provided code

---

**Test Duration**: ~20-30 seconds  
**Resource Requirements**: Moderate (4 validator nodes)  
**Complexity**: Intermediate - demonstrates network partition scenarios
