# Antithesis Docker Config Generator

This directory, along with the script located at the upper level `antithesis_docker_gen.sh`, is used to conveniently generate the Docker files needed to spin up a custom network composed of a specified number of VN, VFN, the faucet service, and a client that contains tests in Antithesis format.

## Generate Config

Run `./scripts/antithesis_docker_gen.sh -i 192.168.5.0 -n 4 -b <branch> -d <target dir>` from the root folder of aptos-core.

Once everything is generated, you can manually modify the configuration for quick tuning. Remember that the generation process involves building and instrumenting some binaries, which are built for the same platform as the Docker node. Once the binaries are built and the configurations are generated, three separate images must be built and uploaded to the Antithesis Docker registry.

> NOTE: If you are using a node release branch, the version (e.g., 1.26.0) is used as a tag.

```
docker build -f Dockerfile-node -t aptos-node:latest .
docker build -f Dockerfile-config -t config:latest .
docker build -f Dockerfile-client -t aptos-client:latest .
```
You should also push to the registry--since internet connection is not available:
```
postgress:14.0
aptos/indexer-processor-rust:latest
hasura/graphql-engine:latest
```
The script already pulls the images, uses tags, and pushes them to make them available, removing aptos/ and hasura/ in order for them to be inside your org.

Refer to https://antithesis.com/docs/getting_started/deploy_to_antithesis/this link for instructions on pushing images into the registry and startstarting a test run.

## Add Tests

The tests are written in TypeScript and are hosted in a separated repository. Inside the /aptos-core/scripts/antithesis-templates/antithesis-tests directory, wrapper scripts can be found. These scripts, with their specific prefixes and directory structure, define the testing strategy for the Antithesis platform.