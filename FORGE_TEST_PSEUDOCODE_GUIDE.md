# Forge Test Framework: Rust to Pseudocode Conversion Guide

This guide shows how to convert Forge test Rust code into generic pseudocode patterns that capture the essential structure and logic.

## 1. Core Test Framework Structure

### Rust Pattern
```rust
// Test trait implementation
impl Test for MyTest {
    fn name(&self) -> &'static str {
        "my_test_name"
    }
}

// NetworkTest trait implementation  
#[async_trait]
impl NetworkTest for MyTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        // Test logic here
    }
}
```

### Generic Pseudocode
```
CLASS TestInterface:
    METHOD name() -> STRING
    METHOD ignored() -> BOOLEAN [default: false]
    METHOD should_fail() -> ShouldFail [default: No]

CLASS NetworkTest EXTENDS TestInterface:
    ASYNC METHOD run(context: NetworkContext) -> Result

CLASS MyTest IMPLEMENTS NetworkTest:
    METHOD name() -> STRING:
        RETURN "my_test_name"
    
    ASYNC METHOD run(context: NetworkContext) -> Result:
        // Test execution logic
        RETURN SUCCESS
```

## 2. Test Configuration Pattern

### Rust Pattern
```rust
fn my_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_fullnode_count(0)
        .add_network_test(MyTest::default())
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.round_initial_timeout_ms = 2000;
        }))
        .with_success_criteria(
            SuccessCriteria::new(5)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
        )
}
```

### Generic Pseudocode
```
FUNCTION create_test_suite() -> TestConfig:
    config = TestConfig()
    config.initial_validator_count = 4
    config.initial_fullnode_count = 0
    config.add_test(MyTest())
    config.validator_config_override = FUNCTION(config):
        config.consensus.timeout = 2000
    config.success_criteria = SuccessCriteria(
        min_tps = 5,
        no_restarts = true,
        catchup_timeout = 60
    )
    RETURN config
```

## 3. Network Context and Swarm Access

### Rust Pattern
```rust
async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
    let swarm = {
        let ctx_guard = ctx.ctx.lock().await;
        ctx_guard.swarm.clone()
    };
    
    let validator_clients = {
        swarm.read().await.get_validator_clients_with_names()
    };
    
    let public_info = {
        swarm.read().await.aptos_public_info()
    };
}
```

### Generic Pseudocode
```
ASYNC METHOD run(context: NetworkContext) -> Result:
    swarm = context.get_swarm()
    validator_clients = swarm.get_validator_clients()
    public_info = swarm.get_public_info()
    
    // Validate test requirements
    IF validator_clients.length != 4:
        RETURN ERROR("Requires exactly 4 validators")
```

## 4. Failure Injection Pattern

### Rust Pattern
```rust
#[async_trait]
impl FailureInjection for MyFailureInjection {
    async fn inject(
        &mut self,
        validator_clients: &[(String, RestClient)],
        cycle: usize,
        part: usize,
    ) {
        match (cycle, part) {
            (0, 0) => {
                // Round 1: Normal operation
                self.remove_partition().await;
            },
            (1, 0) => {
                // Round 2: Apply partition
                self.apply_partition().await;
            },
            _ => {
                // Handle other cases
            }
        }
    }
    
    async fn clear(&mut self, _validator_clients: &[(String, RestClient)]) {
        self.remove_all_chaos().await;
    }
}
```

### Generic Pseudocode
```
INTERFACE FailureInjection:
    ASYNC METHOD inject(clients: List[Client], cycle: INT, part: INT)
    ASYNC METHOD clear(clients: List[Client])

CLASS MyFailureInjection IMPLEMENTS FailureInjection:
    ASYNC METHOD inject(clients: List[Client], cycle: INT, part: INT):
        SWITCH (cycle, part):
            CASE (0, 0):
                // Round 1: Normal operation
                CALL remove_partition()
            CASE (1, 0):
                // Round 2: Apply partition
                CALL apply_partition()
            DEFAULT:
                // Handle unexpected cases
                LOG("Unexpected cycle/part: " + cycle + "/" + part)
    
    ASYNC METHOD clear(clients: List[Client]):
        CALL remove_all_chaos()
```

## 5. Network Chaos Operations

### Rust Pattern
```rust
async fn apply_partition(&mut self) -> Result<()> {
    let group_netems = vec![
        GroupNetEm {
            name: "partition_a_to_b".to_string(),
            source_nodes: vec![peer_ids[0], peer_ids[1]],
            target_nodes: vec![peer_ids[2], peer_ids[3]],
            delay_latency_ms: 0,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 0,
            loss_percentage: 100, // 100% packet loss = partition
            loss_correlation_percentage: 0,
            rate_in_mbps: 0,
        },
    ];
    
    let chaos = SwarmChaos::NetEm(SwarmNetEm { group_netems });
    self.swarm.write().await.inject_chaos(chaos).await?;
}
```

### Generic Pseudocode
```
ASYNC METHOD apply_partition() -> Result:
    network_rules = [
        NetworkRule(
            name = "partition_a_to_b",
            source_nodes = [peer_ids[0], peer_ids[1]],
            target_nodes = [peer_ids[2], peer_ids[3]],
            delay_ms = 0,
            jitter_ms = 0,
            loss_percentage = 100,  // Complete partition
            rate_mbps = 0
        )
    ]
    
    chaos = NetworkChaos.NetEm(network_rules)
    CALL swarm.inject_chaos(chaos)
    RETURN SUCCESS
```

## 6. Consensus Test Execution Pattern

### Rust Pattern
```rust
test_consensus_fault_tolerance(
    validator_clients,
    public_info,
    2,                           // cycles (rounds)
    self.round_duration_secs,    // duration per cycle
    1,                           // parts per cycle
    scenario_injection,
    Box::new(move |cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state| {
        // Progress validation logic
        match cycle {
            0 => {
                // Validate Round 1
                let progress: Vec<u64> = current_state.iter()
                    .zip(previous_state.iter())
                    .map(|(curr, prev)| curr.round.saturating_sub(prev.round))
                    .collect();
                
                if !progress.iter().all(|&p| p > 0) {
                    return Err(anyhow!("Not all nodes made progress"));
                }
            },
            1 => {
                // Validate Round 2
                // Check partition effects
            },
            _ => {}
        }
        Ok(())
    }),
    false, // new_epoch_on_cycle
    false, // raise_check_error_at_the_end
).await?;
```

### Generic Pseudocode
```
CALL test_consensus_fault_tolerance(
    clients = validator_clients,
    public_info = public_info,
    cycles = 2,
    cycle_duration = round_duration_secs,
    parts_per_cycle = 1,
    failure_injection = scenario_injection,
    progress_validator = FUNCTION(cycle, epochs, rounds, transactions, current_state, previous_state):
        SWITCH cycle:
            CASE 0:
                // Round 1: Normal operation validation
                progress = []
                FOR i = 0 TO current_state.length:
                    progress[i] = current_state[i].round - previous_state[i].round
                
                IF NOT ALL(progress > 0):
                    RETURN ERROR("Not all nodes made progress in Round 1")
                    
            CASE 1:
                // Round 2: Partition effects validation
                LOG("Validating partition effects")
                // Check if nodes show expected behavior during partition
                
            DEFAULT:
                LOG("Unexpected round: " + cycle)
        
        RETURN SUCCESS
    ),
    new_epoch_on_cycle = false,
    raise_error_at_end = false
)
```

## 7. State Monitoring and Validation

### Rust Pattern
```rust
let mut round_states = Vec::new();

// In progress validator
Box::new(move |cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state| {
    info!(
        "Round {} completed: epochs={}, rounds={}, transactions={}",
        cycle + 1, executed_epochs, executed_rounds, executed_transactions
    );
    
    // Log node states
    for (i, state) in current_state.iter().enumerate() {
        info!(
            "Node {}: version={}, epoch={}, round={}",
            i, state.version, state.epoch, state.round
        );
    }
    
    // Store states for analysis
    round_states.push((cycle, current_state.clone(), previous_state.clone()));
    
    Ok(())
})
```

### Generic Pseudocode
```
round_states = []

FUNCTION progress_validator(cycle, epochs, rounds, transactions, current_state, previous_state):
    LOG("Round " + (cycle + 1) + " completed: epochs=" + epochs + 
        ", rounds=" + rounds + ", transactions=" + transactions)
    
    // Log individual node states
    FOR i = 0 TO current_state.length:
        state = current_state[i]
        LOG("Node " + i + ": version=" + state.version + 
            ", epoch=" + state.epoch + ", round=" + state.round)
    
    // Store for analysis
    round_states.append((cycle, current_state, previous_state))
    
    RETURN SUCCESS
```

## 8. Test Suite Integration Pattern

### Rust Pattern
```rust
// In lib.rs
pub mod my_test;

// In ungrouped.rs
use aptos_testcases::my_test::MyTest;

fn get_ungrouped_test(name: &str) -> Option<ForgeConfig> {
    match name {
        "my_test" => Some(my_test_suite()),
        _ => None,
    }
}

fn my_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(MyTest::default())
}
```

### Generic Pseudocode
```
// Module registration
MODULE my_test

// Test suite registry
FUNCTION get_test_suite(name: STRING) -> TestConfig:
    SWITCH name:
        CASE "my_test":
            RETURN create_my_test_suite()
        DEFAULT:
            RETURN NULL

FUNCTION create_my_test_suite() -> TestConfig:
    config = TestConfig()
    config.initial_validator_count = 4
    config.add_test(MyTest())
    RETURN config
```

## 9. Error Handling and Cleanup

### Rust Pattern
```rust
async fn clear(&mut self, _validator_clients: &[(String, RestClient)]) {
    info!("Cleaning up - removing all network chaos");
    if let Err(e) = self.remove_partition().await {
        eprintln!("Failed to clear partition: {:?}", e);
    }
}

// In main test
.await?; // Propagate errors

// Final reporting
ctx.report_text(format!(
    "Test completed:\n- Round 1: Normal operation\n- Round 2: Partition applied"
)).await;
```

### Generic Pseudocode
```
ASYNC METHOD clear(clients: List[Client]):
    LOG("Cleaning up - removing all network chaos")
    TRY:
        CALL remove_partition()
    CATCH error:
        LOG_ERROR("Failed to clear partition: " + error)

// Main test execution
TRY:
    CALL test_consensus_fault_tolerance(...)
CATCH error:
    RETURN ERROR("Test failed: " + error)

// Final reporting
context.report_text(
    "Test completed:\n" +
    "- Round 1: Normal operation\n" +
    "- Round 2: Partition applied"
)
```

## 10. Complete Test Structure Pseudocode

```
CLASS FourNodePartitionTest IMPLEMENTS NetworkTest:
    PROPERTIES:
        round_duration_secs: FLOAT = 10.0
    
    METHOD name() -> STRING:
        RETURN "four_node_partition_scenario"
    
    ASYNC METHOD run(context: NetworkContext) -> Result:
        // 1. Setup
        swarm = context.get_swarm()
        validator_clients = swarm.get_validator_clients()
        public_info = swarm.get_public_info()
        
        // 2. Validation
        IF validator_clients.length != 4:
            RETURN ERROR("Requires exactly 4 validators")
        
        // 3. Get network topology
        peer_ids = swarm.get_validator_peer_ids()
        
        // 4. Create failure injection
        failure_injection = FourNodeFailureInjection(swarm, peer_ids)
        
        // 5. Execute test
        CALL test_consensus_fault_tolerance(
            clients = validator_clients,
            public_info = public_info,
            cycles = 2,
            cycle_duration = round_duration_secs,
            parts_per_cycle = 1,
            failure_injection = failure_injection,
            progress_validator = create_progress_validator(),
            new_epoch_on_cycle = false,
            raise_error_at_end = false
        )
        
        // 6. Report results
        context.report_text("Four-node partition scenario completed")
        RETURN SUCCESS

CLASS FourNodeFailureInjection IMPLEMENTS FailureInjection:
    PROPERTIES:
        swarm: Swarm
        peer_ids: List[PeerId]
        partition_applied: BOOLEAN = false
    
    ASYNC METHOD inject(clients: List[Client], cycle: INT, part: INT):
        SWITCH (cycle, part):
            CASE (0, 0):
                CALL remove_partition()  // Round 1: Normal
            CASE (1, 0):
                CALL apply_partition()   // Round 2: Partition
            DEFAULT:
                LOG("Unexpected cycle/part")
    
    ASYNC METHOD clear(clients: List[Client]):
        CALL remove_all_chaos()
    
    ASYNC METHOD apply_partition():
        network_rules = create_partition_rules()
        chaos = NetworkChaos.NetEm(network_rules)
        CALL swarm.inject_chaos(chaos)
        partition_applied = true
    
    ASYNC METHOD remove_partition():
        IF partition_applied:
            CALL swarm.remove_all_chaos()
            partition_applied = false

FUNCTION create_progress_validator() -> Function:
    RETURN FUNCTION(cycle, epochs, rounds, transactions, current_state, previous_state):
        LOG("Round " + (cycle + 1) + " completed")
        
        // Log node states
        FOR i = 0 TO current_state.length:
            state = current_state[i]
            LOG("Node " + i + ": round=" + state.round)
        
        // Validate progress based on round
        SWITCH cycle:
            CASE 0:
                // Round 1: All nodes should progress
                progress = calculate_progress(current_state, previous_state)
                IF NOT ALL(progress > 0):
                    RETURN ERROR("No progress in Round 1")
            CASE 1:
                // Round 2: Check partition effects
                LOG("Partition effects observed")
        
        RETURN SUCCESS
```

## Key Conversion Principles

1. **Traits → Interfaces**: Convert Rust traits to generic interfaces
2. **Async/Await → ASYNC/AWAIT**: Maintain asynchronous patterns
3. **Error Handling**: Convert `Result<T>` to explicit error handling
4. **Ownership → References**: Convert Rust ownership to reference passing
5. **Pattern Matching → Switch Statements**: Convert match expressions
6. **Closures → Functions**: Convert closures to named functions
7. **Type Safety → Type Annotations**: Add explicit type information
8. **Memory Management → Garbage Collection**: Assume automatic memory management

This pseudocode captures the essential logic and structure while being language-agnostic and easier to understand for those not familiar with Rust.



