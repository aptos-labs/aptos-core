# aptos-fullnode

![Version: 1.0.0](https://img.shields.io/badge/Version-1.0.0-informational?style=flat-square) ![AppVersion: 1.0.0](https://img.shields.io/badge/AppVersion-1.0.0-informational?style=flat-square)

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| affinity | object | `{}` |  |
| aptos_chains | object | `{"devnet":{"genesis_blob_url":"https://devnet.aptoslabs.com/genesis.blob","waypoint_txt_url":"https://devnet.aptoslabs.com/waypoint.txt"},"mainnet":{"genesis_blob_url":"https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/genesis.blob","waypoint_txt_url":"https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/waypoint.txt"},"testnet":{"genesis_blob_url":"https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/testnet/genesis.blob","waypoint_txt_url":"https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/testnet/waypoint.txt"}}` | For each supported chain, specify the URLs from which to download the genesis.blob and waypoint.txt |
| backup.affinity | object | `{}` |  |
| backup.config.azure.account | string | `nil` |  |
| backup.config.azure.container | string | `nil` |  |
| backup.config.azure.sas | string | `nil` |  |
| backup.config.gcs.bucket | string | `nil` |  |
| backup.config.location | string | `nil` | Which of the below backup configurations to use |
| backup.config.s3.bucket | string | `nil` |  |
| backup.config.state_snapshot_interval_epochs | int | `1` | State snapshot interval epochs |
| backup.config.transaction_batch_size | int | `1000000` | Transaction batch size |
| backup.enable | bool | `false` | Whether to enable backup |
| backup.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for backup images |
| backup.image.repo | string | `"aptoslabs/tools"` | Image repo to use for backup images |
| backup.image.tag | string | `nil` | Image tag to use for backup images |
| backup.nodeSelector | object | `{}` |  |
| backup.resources.limits.cpu | int | `1` |  |
| backup.resources.limits.memory | string | `"1Gi"` |  |
| backup.resources.requests.cpu | int | `1` |  |
| backup.resources.requests.memory | string | `"1Gi"` |  |
| backup.tolerations | list | `[]` |  |
| backup_verify.affinity | object | `{}` |  |
| backup_verify.nodeSelector | object | `{}` |  |
| backup_verify.resources.limits.cpu | int | `4` |  |
| backup_verify.resources.limits.memory | string | `"4Gi"` |  |
| backup_verify.resources.requests.cpu | int | `4` |  |
| backup_verify.resources.requests.memory | string | `"4Gi"` |  |
| backup_verify.schedule | string | `"@daily"` | The schedule for backup verification |
| backup_verify.tolerations | list | `[]` |  |
| chain.era | int | `1` | Bump this number to wipe the underlying storage |
| chain.genesisConfigmap | string | `nil` | Kubernetes Configmap from which to load the genesis.blob and waypoint.txt |
| chain.genesisSecret | string | `nil` | Kubernetes Secret from which to load the genesis.blob and waypoint.txt |
| chain.name | string | `"devnet"` | Name of the testnet to connect to. There must be a corresponding entry in .Values.aptos_chains |
| fullnode.config | object | `{"full_node_networks":[{"identity":{},"inbound_rate_limit_config":null,"max_inbound_connections":100,"network_id":"public","outbound_rate_limit_config":null,"seeds":{}}]}` | Fullnode configuration. See NodeConfig https://github.com/aptos-labs/aptos-core/blob/main/config/src/config/mod.rs |
| image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for fullnode images |
| image.repo | string | `"aptoslabs/validator"` | Image repo to use for fullnode images. Fullnodes and validators use the same image |
| image.tag | string | `nil` | Image tag to use for fullnode images. If set, overrides `imageTag` |
| imageTag | string | `"devnet"` | Default image tag to use for all fullnode images |
| ingress.annotations | object | `{}` |  |
| ingress.enabled | bool | `false` | Change enabled to true and fill out the rest of the fields to expose the REST API externally via your ingress controller |
| ingress.hostName | string | `nil` | The hostname to use for the ingress |
| ingress.ingressClassName | string | `nil` | The ingress class for fullnode ingress. Leaving class empty will result in an ingress that implicity uses the default ingress class |
| logging.address | string | `nil` | Address for remote logging |
| manageImages | bool | `true` | If true, helm will always override the deployed image with what is configured in the helm values. If not, helm will take the latest image from the currently running workloads, which is useful if you have a separate procedure to update images (e.g. rollout) |
| nodeSelector | object | `{}` |  |
| resources.limits.cpu | int | `14` |  |
| resources.limits.memory | string | `"26Gi"` |  |
| resources.requests.cpu | int | `14` |  |
| resources.requests.memory | string | `"26Gi"` |  |
| restore.affinity | object | `{}` |  |
| restore.config.azure.account | string | `nil` |  |
| restore.config.azure.container | string | `nil` |  |
| restore.config.azure.sas | string | `nil` |  |
| restore.config.concurrent_downloads | int | `2` | Number of concurrent downloads for restore |
| restore.config.gcs.bucket | string | `nil` |  |
| restore.config.location | string | `nil` | Which of the below backup configurations to use |
| restore.config.restore_era | string | `nil` | If set, specifies a different era to restore other than the default era set in chain.era |
| restore.config.s3.bucket | string | `nil` |  |
| restore.config.trusted_waypoints | list | `[]` | List of trusted waypoints for restore |
| restore.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for restore images |
| restore.image.repo | string | `"aptoslabs/tools"` | Image repo to use for restore images |
| restore.image.tag | string | `nil` | Image tag to use for restore images |
| restore.nodeSelector | object | `{}` |  |
| restore.resources.limits.cpu | int | `6` |  |
| restore.resources.limits.memory | string | `"15Gi"` |  |
| restore.resources.requests.cpu | int | `6` |  |
| restore.resources.requests.memory | string | `"15Gi"` |  |
| restore.tolerations | list | `[]` |  |
| rust_log | string | `"info"` | Log level for the fullnode |
| service.annotations | object | `{}` |  |
| service.exposeApi | bool | `true` | Whether to expose the node REST API |
| service.externalTrafficPolicy | string | `nil` | The externalTrafficPolicy for the fullnode service |
| service.loadBalancerSourceRanges | list | `[]` | If set and if the ServiceType is LoadBalancer, allow traffic to fullnode from these CIDRs |
| service.type | string | `"ClusterIP"` | The Kubernetes ServiceType to use for the fullnode. Change this to LoadBalancer expose the REST API, aptosnet endpoint externally |
| serviceAccount.annotations | object | `{}` |  |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `nil` | The name of the service account to use. If not set and create is true, a name is generated using the fullname template |
| storage.class | string | `nil` | Kubernetes storage class to use for fullnode persistent storage |
| storage.size | string | `"1000Gi"` | Size of fullnode persistent storage |
| tolerations | list | `[]` |  |

Configuration
-------------

This Helm chart deploys a public fullnode for the Aptos blockchain network. The
fullnode connects to Aptos validators and synchronises the blockchain state to
a persistent volume. It provides a [REST API][] for interacting with
the blockchain.

See [values.yaml][] for the full list of options you can configure.

Connecting to Testnet
-------------

To connect to the Aptos devnet, you must have the correct genesis blob and waypoint. The source of truth for these are hosted here: https://github.com/aptos-labs/aptos-genesis-waypoint

The waypoint and genesis blobs are download at runtime, and their URLs are specified in `.Values.aptos_chains`.

Deployment
----------

1. Install Helm v3: https://helm.sh/docs/intro/install/
2. Configure `kubectl` with the Kubernetes cluster you wish to use.
3. Install the release, setting any options:

       $ helm install fullnode --set storage.class=gp2 .

[REST API]: https://github.com/aptos-labs/aptos-core/blob/main/api/doc/v0/openapi.yaml
[values.yaml]: values.yaml
[Aptos dockerhub]: https://hub.docker.com/r/aptoslabs/validator/tags?page=1&ordering=last_updated
