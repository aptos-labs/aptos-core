# Telemetry Test Client

A CLI tool to test the Aptos Telemetry Service custom contract endpoints.

## Building

```bash
cd /path/to/aptos-core
cargo build -p telemetry-test-client
```

## Usage

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TELEMETRY_SERVICE_URL` | Base URL of the telemetry service | `http://localhost:8082` |
| `CONTRACT_NAME` | Custom contract name to use | `e2e_test_contract` |
| `PRIVATE_KEY_HEX` | Private key for signing (hex, with or without 0x) | Random |
| `CHAIN_ID` | Chain ID | `4` |

### Commands

#### Authenticate Only
```bash
# Get a JWT token
cargo run -p telemetry-test-client -- auth

# With specific key
cargo run -p telemetry-test-client -- -p 0x1234...abcd auth
```

#### Send Metrics
```bash
# Send sample metrics
cargo run -p telemetry-test-client -- metrics

# Send metrics from file
cargo run -p telemetry-test-client -- metrics -f /path/to/metrics.prom

# Custom sample metric
cargo run -p telemetry-test-client -- metrics --metric-name my_metric --metric-value 100
```

#### Send Logs
```bash
# Send sample logs
cargo run -p telemetry-test-client -- logs

# Send logs from file (JSON array of strings)
cargo run -p telemetry-test-client -- logs -f /path/to/logs.json

# Custom sample logs
cargo run -p telemetry-test-client -- logs --message "Hello World" --count 5
```

#### Send Custom Events
```bash
# Send sample events
cargo run -p telemetry-test-client -- events

# Send events from file (TelemetryDump JSON format)
cargo run -p telemetry-test-client -- events -f /path/to/events.json

# Custom event name
cargo run -p telemetry-test-client -- events --event-name MY_CUSTOM_EVENT
```

#### Send All Data Types
```bash
# Send all types once
cargo run -p telemetry-test-client -- all

# Send all types with 10 iterations, 2 second delay
cargo run -p telemetry-test-client -- all -i 10 -d 2
```

### Full Example with E2E Test Setup

```bash
# Source the test environment
source /path/to/aptos-core/crates/aptos-telemetry-service/e2e-test/test-data/.env

# Run with the test account key
cargo run -p telemetry-test-client -- \
    -u http://localhost:8082 \
    -c e2e_test_contract \
    -p $TEST_ACCOUNT_KEY_HEX \
    --chain-id 4 \
    all -i 5 -d 1
```

## File Formats

### Metrics File (Prometheus format)
```
# HELP my_metric A sample metric
# TYPE my_metric gauge
my_metric{label="value"} 42.0 1701234567890
```

### Logs File (JSON array)
```json
[
  "{\"level\":\"INFO\",\"message\":\"Log 1\"}",
  "{\"level\":\"WARN\",\"message\":\"Log 2\"}"
]
```

### Events File (TelemetryDump format)
```json
{
  "client_id": "my-client",
  "user_id": "0x123...",
  "timestamp_micros": "1701234567890000",
  "events": [
    {
      "name": "MY_EVENT",
      "params": {
        "key1": "value1"
      }
    }
  ]
}
```

