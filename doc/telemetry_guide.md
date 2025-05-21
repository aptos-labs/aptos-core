# Aptos Telemetry Enhancements

1. **Enhanced Consensus Metrics**:
   - Added metrics for committed blocks and transactions
   - Included consensus round and version information
   - Added metrics for consensus timing and performance
   - Added sync information metrics

2. **Enhanced Transaction Metrics**:
   - Added mempool transaction processing metrics
   - Added metrics for transaction broadcast performance
   - Included pending transaction counts

3. **Enhanced Storage Metrics**:
   - Added latency metrics for transaction retrieval
   - Added latency metrics for transaction commits
   - Added latency metrics for transaction saving

4. **Test Suite**:
   - Added unit tests to verify telemetry metrics collection
   - Added integration tests for telemetry end-to-end testing

## Running Locally

To run and test the telemetry implementation locally:

### 1. Install Prometheus

```bash
brew install prometheus
```

### 2. Install Grafana

```bash
brew install grafana
```

## Configuration

### 1. Prometheus Setup

Create or modify `/opt/homebrew/etc/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'aptos'
    static_configs:
      - targets: ['127.0.0.1:9101']
    metrics_path: '/metrics'
    scheme: 'http'
```

### 2. Start Services

1. Start Prometheus:
```bash
brew services start prometheus
```

2. Start Grafana:
```bash
brew services start grafana
```

### 3. Grafana Dashboard Setup

1. Access Grafana UI:
   - Open `http://localhost:3000` in your browser
   - Default login: 
     - Username: `admin`
     - Password: `admin`

2. Add Prometheus Data Source:
   - Go to Connections (⚙️) > Data Sources
   - Click "Add data source"
   - Select "Prometheus"
   - Set URL to `http://localhost:9090`
   - Click "Save & Test"

3. Import Dashboard:
   - Click the "+" icon in the sidebar
   - Select "Import"
   - Upload the provided `aptos-dashboard.json`
   - Select your Prometheus data source
   - Click "Import"

## Verification

1. Check Prometheus targets:
   - Visit `http://localhost:9090/targets`
   - Verify the Aptos target is "UP"

2. Check Grafana metrics:
   - View the imported dashboard
   - Verify metrics are being displayed
   - Check for:
     - Consensus metrics
     - Storage metrics
     - Mempool metrics
     - State sync metrics

## Troubleshooting

1. If metrics aren't showing:
   - Verify Aptos node is running
   - Check metrics endpoint: `curl http://localhost:9101/metrics`
   - Verify Prometheus target status
   - Check Grafana data source connection

2. Service management:
   ```bash
   # Restart services
   brew services restart prometheus
   brew services restart grafana

   # Check service status
   brew services list

   # View Prometheus logs
   tail -f /opt/homebrew/var/log/prometheus.log

   # View Grafana logs
   tail -f /opt/homebrew/var/log/grafana.log
   ```

## Stopping Services

```bash
brew services stop prometheus
brew services stop grafana
``` 

## Telemetry Metrics

The following key metrics have been added:

### Consensus Metrics

| Metric Name | Description |
|-------------|-------------|
| `consensus_last_committed_version` | The last committed ledger version |
| `consensus_committed_blocks_count` | Number of blocks committed since node start |
| `consensus_committed_txns_count` | Number of transactions committed since node start |
| `consensus_current_round` | Current consensus round |
| `consensus_round_timeout_secs` | Average round timeout in seconds |
| `consensus_sync_info_msg_sent_count` | Number of sync info messages sent |
| `consensus_wait_duration_s` | Average wait duration in seconds |

### Mempool Metrics

| Metric Name | Description |
|-------------|-------------|
| `mempool_txns_processed_success` | Number of successfully processed transactions |
| `mempool_txns_processed_total` | Total number of transactions received |
| `mempool_avg_txn_broadcast_size` | Average transaction broadcast size |
| `mempool_pending_txns` | Number of pending transactions in mempool |

### Storage Metrics

| Metric Name | Description |
|-------------|-------------|
| `storage_get_transaction_latency_s` | Average latency for transaction retrieval |
| `storage_commit_latency_s` | Average latency for transaction commits |
| `storage_save_transactions_latency_s` | Average latency for saving transactions |
