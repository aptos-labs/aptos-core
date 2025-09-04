# All processors are migrated to [processor repo](https://github.com/velor-chain/velor-indexer-processors)

# Indexer GRPC Parser

Indexer GRPC parser is to indexer data processor that leverages the indexer grpc data.



## Tutorial
### Prerequisite
* A running PostgreSQL instance, with a valid database. More tutorial can be found [here](https://github.com/velor-chain/velor-core/tree/main/crates/indexer#postgres)

* A config YAML file
  * For exmaple, `config.yaml`
  * 
    ```yaml
    health_check_port: 8084
    server_config:
        processor_name: default_processor
        postgres_connection_string: postgresql://postgres:@localhost:5432/postgres_v2
        indexer_grpc_data_service_address: 127.0.0.1:50051
        indexer_grpc_http2_ping_interval_in_secs: 60
        indexer_grpc_http2_ping_timeout_in_secs: 10
        auth_token: AUTH_TOKEN
    ```

#### Config Explanation

* `processor_name`: purpose of this processor; also used for monitoring purpose.
* `postgres_connection_string`: PostgresQL DB connection string
* `indexer_grpc_data_service_address`: Data service non-TLS endpoint address.
* `indexer_grpc_http2_ping_interval_in_secs`: client-side grpc HTTP2 ping interval.
* `indexer_grpc_http2_ping_timeout_in_secs`: client-side grpc HTTP2 ping timeout.
* `auth_token`: Auth token used for connection.


### Use docker image for existing parsers(Only for **Unix/Linux**)
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
  * Build: `cd ecosystem/indexer-grpc/indexer-grpc-parser && docker build . -t indexer-processor`
  * Run: `docker run indexer-processor:latest`
  

### Use source code for existing parsers
* Use the provided `Dockerfile` and `config.yaml`(update accordingly)
* Run `cd ecosystem/indexer-grpc/indexer-grpc-parser && cargo run --release -- -c config.yaml`


### Use a custom parser
* Check our [indexer processors](https://github.com/velor-chain/velor-indexer-processors)! 
