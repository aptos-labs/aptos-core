# Docker Images Builder

This directory contains [Docker](https://www.docker.com/) configuration for building Velor docker images. This builder requires the use of Buildkit which is available by default in most recent Docker installations.

To build these images run this from the repository root:

```
docker buildx create --use # creates a buildkit builder and only needs to be run once
docker/builder/docker-bake-rust-all.sh
```

The above command will by default build all the images. To build specific images, refer to `group` and `target` definitions in [docker-bake-rust-all.hcl](docker-bake-rust-all.hcl).

For using the images, look in the [docker/compose](../docker/compose/) directory.

## List of Images

The builder can produce the following Docker images. To build a particular image, run `./docker/builder/docker-bake-rust-all.sh [image-name]`. Also, refer to the `group` definitions in the [docker-bake-rust-all.hcl](docker-bake-rust-all.hcl) file for more information.

1. `validator-testing` : Image containing the `velor-node`, `velor-debugger` binaries and other linux tools useful for debugging and testing. This image is used in Forge tests.
2. `validator` : Image containing the `velor-node` and `velor-debugger` binaries. This image is usually used for distribution.
3. `tools`: Image containing all the velor tools binaries including `velor-debugger`, `velor`, `velor-transaction-emitter`, `velor-openapi-spec-generator` and `velor-fn-check-client`. Also, includes the Velor Move framework for use with genesis generation.
4. `forge`: Image containing the `forge` binary that orchestrates and runs Forge tests.
5. `node-checker`: Image containing the `node-checker` binary that checks the health of a node.
6. `faucet`: Image containing the `faucet` binary that provides a faucet service for minting coins.
7. `indexer-grpc`: Image containing the `indexer-grpc` binary that indexes the blockchain and provides a gRPC service for querying.
8. `telemetry-service`: Image containing the `telemetry-service` binary that collects telemetry from blockchain nodes.

## How the builder works

At a high level, the builder works as follows. By default, the builder builds all images.

1. One of `velor-node-builder`, `indexer-builder`, or `tools-builder` targets are invoked depending on what image is being built.
2. The target image is built by copying the output of either the `velor-node-builder` or `tools-builder` target into the target image.

The `velor-node-builder` is separate from the other builder targets because it allows to build different `velor-node` binary variants with different features and profiles.

Using a builder step allows us to cache the build artifacts and reuse them across different images. Our binaries have a lot of common dependencies, so this is a significant time saver. Furthermore, most `RUN` instructions use a cache mount that allows us to cache the output of the command leading to significant build time improvements.

## Building a new Image

> Note: If building a CLI tool, consider adding it to the `tools` image instead of creating a new image.

> Note: If your requirements don't fit into the instructions below, please reach out to the team for help.

1. Modify the `cargo build` step in `build-tools.sh` to include the new binary.
2. Create a new Dockerfile by cloning an existing target Dockerfile (e.g. `validator.Dockerfile`). When you use a `RUN` instruction, try to use a mount cache as they can improve build times by caching the output of the command.
3. Add the following `FROM` statements to the new Dockerfile depending on whether you need to copy from the `velor-node-builder`, `indexer-builder`, `tools-builder`. This ensures that your image references the required builder images to copy the binaries from. These image references are injected as build contexts at build time. This is defined in the `contexts` field in `_common` target in [docker-bake-rust-all.hcl](docker-bake-rust-all.hcl).

```
FROM node-builder

FROM tools-builder
```

4. In your new Dockerfile, use the COPY command to copy the output of the `velor-node-builder`, `indexer-builder`, `tools-builder` target into the image. For example, to copy the `velor-node` binary into the `validator` image, use the following command:
   ```
   COPY --link --from=node-builder /velor/dist/velor-node /usr/local/bin/
   ```
5. Add a new target definition in [docker-bake-rust-all.hcl](docker-bake-rust-all.hcl) file by copying another target (e.g. `validator`). The target definition should have the following fields:

   - `inherits`
   - `target`: Name of the target. This should be the same as the name of the Dockerfile.
   - `dockerfile`: Path to the Dockerfile.
   - `tags`: Create a unique tag for the image using `generate_tags` function.
   - `cache-from`: Create a unique cache key using `generate_cache_from` function.
   - `cache-to`: Create a unique cache key using `generate_cache_to` function.

6. Optionally, you can create a `group` definition to build multiple tagets at once.

## Image tagging strategy

The `velor-node-builder`, `indexer-builder`, `tools-builder` targets build the `velor-node` binary and the remaining rust binaries, respectively, and is the most expensive. Its output is used by all the other targets that follow.

The `*-builder` itself takes in a few build arguments. Most are build metadata, such as `GIT_SHA` and `GIT_BRANCH`, but others change the build entirely, such as cargo flags `PROFILE` and `FEATURES`. Arguments like these necessitate a different cache to prevent clobbering. The general strategy is to use image tags and cache keys that use these variables. An example image tag might be:

- `performance_failpoints_<GIT_SHA>` -- `performance` profile with `failpoints` feature
- `<GIT_SHA>` -- default `release` profile with no additional features

## Release Images

Image releasing is done automatically using corresponding github workflow jobs or manually using the `docker/release-images.mjs` script.
