# Aptos Debugger Docker Image

Lightweight container image containing the `aptos-debugger` binary for database
backup and restore operations.

## What's Included

- `aptos-debugger` — Aptos database debugging and backup tool

## Use Cases

- **Continuous backup**: Runs `aptos-debugger aptos-db backup continuously` as a
  sidecar alongside an archival node, streaming epoch endings, state snapshots, and
  transactions to S3.
- **Native restore**: Runs `aptos-debugger aptos-db restore bootstrap-db` to restore
  a node's database from continuous backup data in S3.

## Building Locally

From the repository root:

```bash
# Build the binary + container image
just container-build aptos-debugger latest release

# Verify
docker run --rm ghcr.io/movementlabsxyz/aptos-debugger:latest --version
```

## Pushing to GHCR

```bash
# Authenticate (one-time)
docker login ghcr.io -u <username>

# Push
docker push ghcr.io/movementlabsxyz/aptos-debugger:latest
```

## CI

The `build-versions.yaml` workflow automatically builds and pushes this image on
every push to the default branch. The image is tagged with the short git SHA
(e.g., `ghcr.io/movementlabsxyz/aptos-debugger:f24a5bc`).

## Runtime Dependencies

Same shared libraries as the `aptos-node` image:

- libjemalloc.so.2
- libdw.so.1
- librocksdb.so.10
- libssl.so.3
- Standard glibc libraries
