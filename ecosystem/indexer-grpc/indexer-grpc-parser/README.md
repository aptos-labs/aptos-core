# Indexer GRPC Parser

Indexer GRPC parser is to indexer data processor that leverages the indexer grpc data.

* __Note: We'll launch an official endpoint soon; stay tuned!__

## Tutorial
### Prerequisite
* A running PostgreSQL instance, with a valid database. More tutorial can be found [here](https://github.com/aptos-labs/aptos-core/tree/main/crates/indexer#postgres)

* A config YAML file
  * For exmaple, `config.yaml`
  * 
    ```yaml
    health_check_port: 8084
    server_config:
        processor_name: default_processor
        postgres_connection_string: postgresql://postgres:@localhost:5432/postgres_v2
        indexer_grpc_data_service_addresss: 127.0.0.1:50051
        auth_token: AUTH_TOKEN
    ```

### Use docker image for existing parsers(Only for **Unix/Linux**)
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
  * Build: `cd ecosystem/indexer-grpc/indexer-grpc-parser && docker build . -t indexer-processor`
  * Run: `docker run indexer-processor:latest`
  

### Use source code for existing parsers
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
* Run `cd ecosystem/indexer-grpc/indexer-grpc-parser && cargo run --release -- -c config.yaml`


### Use a custom parser
* Check our [indexer processors](https://github.com/aptos-labs/aptos-indexer-processors)! 
