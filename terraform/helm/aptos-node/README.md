# aptos-node

![Version: 0.1.0](https://img.shields.io/badge/Version-0.1.0-informational?style=flat-square) ![AppVersion: 0.1.0](https://img.shields.io/badge/AppVersion-0.1.0-informational?style=flat-square)

Aptos blockchain node deployment

**Homepage:** <https://aptoslabs.com/>

## Source Code

* <https://github.com/aptos-labs/aptos-core>

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| chain.chain_id | int | `4` | Chain ID |
| chain.era | int | `1` | Bump this number to wipe the underlying storage |
| chain.name | string | `"testnet"` | Internal: name of the testnet to connect to |
| fullnode.affinity | object | `{}` |  |
| fullnode.config | object | `{"max_inbound_connections":100}` | Validator configuration. See NodeConfig https://github.com/aptos-labs/aptos-core/blob/main/config/src/config/mod.rs |
| fullnode.groups | list | `[{"name":"fullnode","replicas":1}]` | Specify fullnode groups by `name` and number of `replicas` |
| fullnode.nodeSelector | object | `{}` |  |
| fullnode.resources.limits.cpu | float | `3.5` |  |
| fullnode.resources.limits.memory | string | `"6Gi"` |  |
| fullnode.resources.requests.cpu | float | `3.5` |  |
| fullnode.resources.requests.memory | string | `"6Gi"` |  |
| fullnode.rust_log | string | `"info"` | Log level for the fullnode |
| fullnode.rust_log_remote | string | `"off"` | Remote log level for the fullnode |
| fullnode.storage.class | string | `nil` | Kubernetes storage class to use for fullnode persistent storage |
| fullnode.storage.size | string | `"350Gi"` | Size of fullnode persistent storage |
| fullnode.tolerations | list | `[]` |  |
| haproxy.affinity | object | `{}` |  |
| haproxy.config.send_proxy_protocol | bool | `false` | Whether to send Proxy Protocol v2 |
| haproxy.enabled | bool | `true` | Enable HAProxy deployment in front of validator and fullnodes |
| haproxy.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for HAProxy images |
| haproxy.image.repo | string | `"haproxy"` | Image repo to use for HAProxy images |
| haproxy.image.tag | string | `"2.2.14@sha256:36aa98fff27dcb2d12c93e68515a6686378c783ea9b1ab1d01ce993a5cbc73e1"` | Image tag to use for HAProxy images |
| haproxy.limits.validator.connectionsPerIPPerMin | int | `2` | Limit the number of connections per IP address per minute |
| haproxy.nodeSelector | object | `{}` |  |
| haproxy.replicas | int | `1` | Number of HAProxy replicas |
| haproxy.resources.limits.cpu | float | `1.5` |  |
| haproxy.resources.limits.memory | string | `"2Gi"` |  |
| haproxy.resources.requests.cpu | float | `1.5` |  |
| haproxy.resources.requests.memory | string | `"2Gi"` |  |
| haproxy.tls_secret | string | `nil` | Name of the Kubernetes TLS secret to use for HAProxy |
| haproxy.tolerations | list | `[]` |  |
| imageTag | string | `"devnet"` | Default image tag to use for all validator and fullnode images |
| labels | string | `nil` |  |
| loadTestGenesis | bool | `false` | Load test-data for starting a test network |
| numFullnodeGroups | int | `1` | Total number of fullnode groups to deploy |
| numValidators | int | `1` | Number of validators to deploy |
| podSecurityPolicy | bool | `true` | LEGACY: create PodSecurityPolicy, which exists at the cluster-level |
| service.domain | string | `nil` | If set, the base domain name to use for External DNS |
| service.fullnode.enableMetricsPort | bool | `true` | Enable the metrics port on fullnodes |
| service.fullnode.enableRestApi | bool | `true` | Enable the REST API on fullnodes |
| service.fullnode.external.type | string | `"LoadBalancer"` | The Kubernetes ServiceType to use for fullnodes |
| service.fullnode.externalTrafficPolicy | string | `"Local"` | The externalTrafficPolicy for the fullnode service |
| service.fullnode.loadBalancerSourceRanges | string | `nil` | If set and if the ServiceType is LoadBalancer, allow traffic to fullnodes from these CIDRs |
| service.validator.enableMetricsPort | bool | `true` | Enable the metrics port on the validator |
| service.validator.enableRestApi | bool | `true` | Enable the REST API on the validator |
| service.validator.external.type | string | `"LoadBalancer"` | The Kubernetes ServiceType to use for validator |
| service.validator.externalTrafficPolicy | string | `"Local"` | The externalTrafficPolicy for the validator service |
| service.validator.loadBalancerSourceRanges | string | `nil` | If set and if the ServiceType is LoadBalancer, allow traffic to validators from these CIDRs |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `nil` | The name of the service account to use. If not set and create is true, a name is generated using the fullname template |
| validator.affinity | object | `{}` |  |
| validator.config | object | `{"concurrency_level":8,"enable_ledger_pruner":true,"enable_state_store_pruner":true,"ledger_prune_window":10000000,"ledger_pruning_batch_size":10000,"quorum_store_poll_count":1,"round_initial_timeout_ms":null,"state_store_prune_window":1000000,"state_store_pruning_batch_size":10000,"sync_only":false}` | Validator configuration. See NodeConfig https://github.com/aptos-labs/aptos-core/blob/main/config/src/config/mod.rs |
| validator.enableNetworkPolicy | bool | `true` | Lock down network ingress and egress with Kubernetes NetworkPolicy |
| validator.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for validator images |
| validator.image.repo | string | `"aptoslabs/validator"` | Image repo to use for validator images |
| validator.image.tag | string | `nil` | Image tag to use for validator images. If set, overrides `imageTag` |
| validator.name | string | `nil` | Internal: name of your validator for use in labels |
| validator.nodeSelector | object | `{}` |  |
| validator.resources.limits.cpu | float | `3.5` |  |
| validator.resources.limits.memory | string | `"6Gi"` |  |
| validator.resources.requests.cpu | float | `3.5` |  |
| validator.resources.requests.memory | string | `"6Gi"` |  |
| validator.rust_log | string | `"info"` | Log level for the validator |
| validator.rust_log_remote | string | `"off"` | Remote log level for the validator |
| validator.storage.class | string | `nil` | Kubernetes storage class to use for validator persistent storage |
| validator.storage.size | string | `"350Gi"` | Size of validator persistent storage |
| validator.tolerations | list | `[]` |  |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.11.0](https://github.com/norwoodj/helm-docs/releases/v1.11.0)
