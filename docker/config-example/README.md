# Local Validator Configuration

This directory contains example configuration for running a local validator node.

## Required Files

To run a local validator, you need the following files in your config directory:

| File | Description | Source |
|------|-------------|--------|
| `validator.yaml` | Node configuration | Copy from `validator.yaml.example` and customize |
| `genesis.blob` | Network genesis | Download from network or generate |
| `waypoint.txt` | Network waypoint | Download from network or use genesis waypoint |

## Quick Start

1. **Copy example configuration:**

   ```bash
   mkdir -p ./config
   cp docker/config-example/validator.yaml.example ./config/validator.yaml
   ```

2. **Obtain genesis and waypoint for your target network:**

   For Movement devnet:

   ```bash
   curl -o ./config/genesis.blob https://devnet.movementnetwork.xyz/genesis.blob
   curl -o ./config/waypoint.txt https://devnet.movementnetwork.xyz/waypoint.txt
   ```

   For Movement testnet:

   ```bash
   curl -o ./config/genesis.blob https://testnet.movementnetwork.xyz/genesis.blob
   curl -o ./config/waypoint.txt https://testnet.movementnetwork.xyz/waypoint.txt
   ```

3. **Edit validator.yaml** to set the correct paths and network settings

4. **Start the validator:**

   ```bash
   just start-local-validator
   # OR
   CONFIG_DIR=./config docker compose -f docker/docker-compose.yml up -d
   ```

5. **Verify the node is running:**

   ```bash
   curl http://localhost:8080/v1
   ```

## Configuration Options

### validator.yaml Key Settings

```yaml
base:
  data_dir: /opt/data/aptos
  role: full_node  # or 'validator' for validator nodes
  waypoint:
    from_file: /config/waypoint.txt

execution:
  genesis_file_location: /config/genesis.blob

full_node_networks:
  - network_id: public
    listen_address: /ip4/0.0.0.0/tcp/6181
    # Add seed peers for network connectivity
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CONFIG_DIR` | `./config` | Path to configuration directory |
| `RUST_LOG` | `info` | Rust logging level |
| `RUST_BACKTRACE` | `1` | Enable stack traces |
| `APTOS_IMAGE` | `ghcr.io/movementlabsxyz/aptos-node:latest` | Container image |

## Troubleshooting

### Node not syncing

- Verify network connectivity to seed peers
- Check `genesis.blob` and `waypoint.txt` match the target network
- Review logs: `just validator-logs`

### REST API not responding

- Wait for node startup (can take 30-60 seconds)
- Check health status: `docker inspect aptos-validator --format='{{.State.Health.Status}}'`

### Out of memory

- Increase Docker memory limits
- Edit `docker-compose.yml` to adjust resource limits

### Data persistence issues

- Data is stored in `aptos-validator-data` Docker volume
- To reset: `docker volume rm aptos-validator-data`
