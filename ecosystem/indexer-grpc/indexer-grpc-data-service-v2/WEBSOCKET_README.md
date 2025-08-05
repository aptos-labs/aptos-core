# WebSocket API for indexer-grpc-data-service-v2

This service now supports WebSocket endpoints in addition to the existing gRPC streaming endpoints. The WebSocket endpoints provide the same functionality but use JSON serialization instead of protobuf, making them more accessible for web clients and other applications that prefer JSON.

## Endpoints

### Transaction Stream
- **URL**: `ws://localhost:8000/ws/transactions`
- **Purpose**: Stream transaction data from the Aptos blockchain

### Event Stream  
- **URL**: `ws://localhost:8000/ws/events`
- **Purpose**: Stream event data with transaction metadata from the Aptos blockchain

## Configuration

The service can run in either gRPC mode or WebSocket mode, but not both simultaneously. Configure the service mode in your configuration:

```yaml
service_config:
  # For WebSocket mode:
  type: "websocket"
  listen_address: "0.0.0.0:8000"
  
  # For gRPC mode:
  # type: "grpc"  
  # listen_address: "0.0.0.0:50051"
  # tls_config:  # Optional for gRPC
  #   cert_path: "/path/to/cert.pem"
  #   key_path: "/path/to/key.pem"
```

## Usage

### 1. Connect to WebSocket
Connect to one of the endpoints using any WebSocket client.

### 2. Send Initial Request
After connecting, send a JSON request to specify what data you want:

#### For Transactions:
```json
{
  "starting_version": 1,
  "transactions_count": 100,
  "batch_size": 10,
  "transaction_filter": null
}
```

#### For Events:
```json
{
  "starting_version": 1,
  "transactions_count": 100,
  "batch_size": 10,
  "transaction_filter": null
}
```

### 3. Receive Streaming Data
The server will respond with a stream of JSON messages. Each message has a `type` field indicating the message type:

#### Transaction Response:
```json
{
  "type": "transactions_response",
  "transactions": [...],
  "chain_id": 1,
  "processed_range": {
    "first_version": 1,
    "last_version": 10
  }
}
```

#### Event Response:
```json
{
  "type": "events_response",
  "events": [
    {
      "event": {...},
      "timestamp": {...},
      "version": 1,
      "hash": "0x...",
      "success": true,
      "vm_status": "Executed successfully",
      "block_height": 1
    }
  ],
  "chain_id": 1,
  "processed_range": {
    "first_version": 1,
    "last_version": 10
  }
}
```

#### Error Response:
```json
{
  "type": "error",
  "message": "Error description"
}
```

#### Stream End:
```json
{
  "type": "stream_end"
}
```

## Request Parameters

### Common Parameters
- `starting_version` (optional): Start version of current stream
- `transactions_count` (optional): Number of transactions to return/process. If not present, returns an infinite stream
- `batch_size` (optional): Number of transactions/events in each response. Default: 1000, Max: 1000  
- `transaction_filter` (optional): Filter for which transactions to include

### Transaction Filters
The `transaction_filter` field supports the same filtering capabilities as the gRPC API:

```json
{
  "transaction_filter": {
    "filter": {
      "api_filter": {
        "filter": {
          "transaction_root_filter": {
            "success": true,
            "transaction_type": 1
          }
        }
      }
    }
  }
}
```

## Client Examples

### JavaScript/Node.js
See `websocket_client_example.js` for a complete example.

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8000/ws/transactions');

ws.on('open', function() {
    const request = {
        starting_version: 1,
        transactions_count: 10,
        batch_size: 5
    };
    ws.send(JSON.stringify(request));
});

ws.on('message', function(data) {
    const response = JSON.parse(data);
    console.log('Received:', response.type);
});
```

### Python
```python
import asyncio
import websockets
import json

async def connect():
    uri = "ws://localhost:8000/ws/transactions"
    async with websockets.connect(uri) as websocket:
        # Send request
        request = {
            "starting_version": 1,
            "transactions_count": 10,
            "batch_size": 5
        }
        await websocket.send(json.dumps(request))
        
        # Receive responses
        while True:
            message = await websocket.recv()
            response = json.loads(message)
            print(f"Received: {response['type']}")
            
            if response["type"] == "stream_end":
                break

asyncio.run(connect())
```

### curl (for testing)
```bash
# Install websocat first: cargo install websocat
echo '{"starting_version": 1, "transactions_count": 5}' | websocat ws://localhost:8000/ws/transactions
```

## Architecture

The WebSocket implementation reuses the existing gRPC streaming infrastructure:

1. WebSocket requests are converted to gRPC requests
2. The existing gRPC stream is consumed internally  
3. gRPC responses are converted to JSON and sent over WebSocket
4. This ensures consistency between gRPC and WebSocket APIs

## Error Handling

- Connection errors are handled gracefully
- Malformed requests result in error responses
- gRPC stream errors are converted to WebSocket error messages
- Serialization errors are caught and reported

## Performance Considerations

- JSON serialization adds overhead compared to protobuf
- WebSocket connections consume server resources
- Consider connection limits and rate limiting for production use
- The same filtering and batching optimizations apply as with gRPC

## CORS Support

The WebSocket server includes permissive CORS headers to allow web browser connections from any origin.