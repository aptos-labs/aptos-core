# Indexer GRPC Docker Compose

This docker-compose is meant to be run in tandem with the `validator-testnet`:
* In `docker/compose/validator-testnet`, start the node: `docker-compose up -d`
  * After it's spun up, you should be able to access its REST API and gRPC endpoints. See the logs and node config file for more details
* In `docker/compose/indexer-grpc`, start the indexer GRPC setup: `docker-compose up -d`

After this point, you will have the following:
* Single validator testnet
* Redis
* Indexer GRPC Cache Worker
* Indexer GRPC File Store (writing to docker volume)
* Indexer GRPC Data Service

Relevant ports are exposed on the docker host for testing purposes

## Reset

A simple script is provided to kill and remove all relevant docker containers and volumes, to reset the whole localnet and indexer setup:

```
./reset_indexer_grpc_testnet.sh
```
