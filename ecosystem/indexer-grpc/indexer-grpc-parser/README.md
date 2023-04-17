# Indexer GRPC Parser

Indexer GRPC parser is to indexer data processor that leverages the indexer grpc data.

* __Note: We'll launch an official endpoint soon; stay tuned!__

## Tutorial
### Prerequisite
* A running PostgreSQL instance, with a valid database. More tutorial can be found [here](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer#postgres)
  * For example,
    * Hostname: `the.postgres.host.name`
    * Port: `5432`
    * User: `dev`
    * Password: `xxxxxxxx`
    * Database name: `indexer_grpc`
* A remote GRPC endpoint
  * For example,
    * Hostname: `the.grpc.hostname`
    * Port: `50050`
* A config YAML file
  * For exmaple, `config.yaml`
  * 
    ```
    indexer_grpc_address: the.grpc.hostname:50050
    postgres_connection_string: postgres://dev:xxxxxxxx@the.postgres.host.name:5432/indexer_grpc
    number_concurrent_processing_tasks: 20
    processor_name: default_processor
    health_check_port: 8080
    indexer_grpc_auth_token: AUTH_TOKEN
    ```

### Use docker image for existing parsers(Only for Unix/Linux)
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
  * Build: `cd ecosystem/indexer-grpc/indexer-grpc-parser && docker build . -t indexer-processor`
  * Run: `docker run indexer-processor:latest`
  

### Use source code for existing parsers
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
* Run `cd ecosystem/indexer-grpc/indexer-grpc-parser && cargo run --release -- -c config.yaml`


### Use a custom parser
WIP
