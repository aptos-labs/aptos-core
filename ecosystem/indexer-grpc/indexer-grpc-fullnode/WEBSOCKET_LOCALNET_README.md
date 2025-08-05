# WebSocket Support for Localnet Transaction and Event Streams

This document describes the native WebSocket support added to the Aptos localnet for streaming transactions and events.

## Overview

The localnet now supports WebSocket streaming alongside the existing gRPC streaming. When you enable the transaction stream service in localnet (via `--with-faucet`), both gRPC and WebSocket servers will be started automatically.

## Configuration

WebSocket support is automatically enabled when you run localnet with transaction streaming:

```bash
aptos node run-localnet --with-faucet
```

**Default Ports:**
- gRPC server: `50051`
- WebSocket server: `50052` (gRPC port + 1)

## WebSocket Endpoints

The WebSocket server exposes two endpoints:

- `/ws/transactions` - Stream transactions
- `/ws/events` - Stream events

## Usage Example

### Transactions Stream

Connect to `ws://127.0.0.1:50052/ws/transactions` and send a JSON request:

```json
{
  "type": "get_transactions",
  "starting_version": 0,
  "transactions_count": 100,
  "batch_size": 10
}
```

**Response format:**
```json
{
  "type": "transactions_response",
  "chain_id": 4,
  "transactions": [...],
  "processed_range": null
}
```

### Events Stream  

Connect to `ws://127.0.0.1:50052/ws/events` and send a JSON request:

```json
{
  "type": "get_events", 
  "starting_version": 0,
  "transactions_count": 100,
  "batch_size": 10
}
```

**Response format:**
```json
{
  "type": "events_response",
  "events": [...],
  "chain_id": 4,
  "processed_range": null
}
```

### Error Handling

If an error occurs, you'll receive an error message:

```json
{
  "type": "error",
  "message": "Error description"
}
```

### Stream End

When the stream completes, you'll receive:

```json
{
  "type": "stream_end"
}
```

## Request Parameters

**Common Parameters:**
- `starting_version` (optional): Start version of the stream (default: 0)
- `transactions_count` (optional): Number of transactions to return (default: infinite stream)
- `batch_size` (optional): Number of transactions/events per response (default: depends on node config)

**Note:** Transaction filtering (`transaction_filter`) is not implemented in the localnet WebSocket service for simplicity, as it's intended for testing and development purposes only.

## Configuration Details

The WebSocket server configuration is automatically set when indexer gRPC is enabled:

```rust
// In config
node_config.indexer_grpc.websocket_enabled = true;
node_config.indexer_grpc.websocket_address = "127.0.0.1:50052";
```

## JavaScript Example

```javascript
const ws = new WebSocket('ws://127.0.0.1:50052/ws/transactions');

ws.onopen = function() {
    // Request transactions starting from version 0
    ws.send(JSON.stringify({
        type: "get_transactions",
        starting_version: 0,
        transactions_count: 10,
        batch_size: 5
    }));
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
    
    if (data.type === 'transactions_response') {
        console.log('Transactions:', data.transactions);
    } else if (data.type === 'error') {
        console.error('Error:', data.message);
    } else if (data.type === 'stream_end') {
        console.log('Stream ended');
        ws.close();
    }
};

ws.onerror = function(error) {
    console.error('WebSocket error:', error);
};
```

## Differences from Data Service WebSocket

The localnet WebSocket implementation:
- Uses the same message format as the full data service
- Does not support transaction filtering (for simplicity)
- Is optimized for testing/development use
- Runs natively on the node without requiring a separate data service