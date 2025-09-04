# velor-node

![Version: 1.0.0](https://img.shields.io/badge/Version-1.0.0-informational?style=flat-square) ![AppVersion: 1.0.0](https://img.shields.io/badge/AppVersion-1.0.0-informational?style=flat-square)

Velor blockchain node deployment

**Homepage:** <https://velorlabs.com/>

## Source Code

* <https://github.com/velor-chain/velor-core>

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| chain.chain_id | int | `4` | Chain ID |
| chain.era | int | `1` | Bump this number to wipe the underlying storage |
| chain.name | string | `"testnet"` | Internal: name of the testnet to connect to |
| cluster_name | string | `"unknown"` |  |
| enablePrivilegedMode | bool | `false` | TEST ONLY: Enable running as root for profiling |
| fullnode.affinity | object | `{}` |  |
| fullnode.config | object | `{"full_node_networks":[{"network_id":"public"}]}` | Fullnode configuration. See NodeConfig https://github.com/velor-chain/velor-core/blob/main/config/src/config/mod.rs |
| fullnode.force_enable_telemetry | bool | `false` | Flag to force enable telemetry service (useful for forge tests) |
| fullnode.groups | list | `[{"dns_name":"vfn","name":"fullnode","replicas":1}]` | Specify fullnode groups by `name` and number of `replicas` |
| fullnode.nodeSelector | object | `{}` |  |
| fullnode.podAnnotations | string | `nil` |  |
| fullnode.resources.limits.cpu | int | `30` |  |
| fullnode.resources.limits.memory | string | `"60Gi"` |  |
| fullnode.resources.requests.cpu | int | `30` |  |
| fullnode.resources.requests.memory | string | `"60Gi"` |  |
| fullnode.rust_log | string | `"info"` | Log level for the fullnode |
| fullnode.storage.class | string | `nil` | Kubernetes storage class to use for fullnode persistent storage |
| fullnode.storage.size | string | `"2048Gi"` | Size of fullnode persistent storage |
| fullnode.tolerations | list | `[]` |  |
| genesis_blob_upload_url | string | `"https://us-west1-velor-forge-gcp-0.cloudfunctions.net/signed-url?cluster_name=unknown&era=1"` |  |
| haproxy.affinity | object | `{}` |  |
| haproxy.config.send_proxy_protocol | bool | `false` | Whether to send Proxy Protocol v2 |
| haproxy.config.user | string | `"nobody"` | System user to run HA |
| haproxy.enabled | bool | `true` | Enable HAProxy deployment in front of validator and fullnodes |
| haproxy.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for HAProxy images |
| haproxy.image.repo | string | `"haproxy"` | Image repo to use for HAProxy images |
| haproxy.image.tag | string | `"3.0.2@sha256:3fa2e323a2f422239a39eff345b41ab20a7a91aa4ad8c3c82b9ae85dd241214b"` | Image tag to use for HAProxy images |
| haproxy.nodeSelector | object | `{}` |  |
| haproxy.replicas | int | `1` | Number of HAProxy replicas |
| haproxy.resources.limits.cpu | int | `7` |  |
| haproxy.resources.limits.memory | string | `"16Gi"` |  |
| haproxy.resources.requests.cpu | int | `7` |  |
| haproxy.resources.requests.memory | string | `"16Gi"` |  |
| haproxy.tls_secret | string | `nil` | Name of the Kubernetes TLS secret to use for HAProxy |
| haproxy.tolerations | list | `[]` |  |
| imageTag | string | `"devnet"` | Default image tag to use for all validator and fullnode images |
| labels | string | `nil` |  |
| loadTestGenesis | bool | `false` | Load test-data for starting a test network |
| manageImages | bool | `true` | If true, helm will always override the deployed image with what is configured in the helm values. If not, helm will take the latest image from the currently running workloads, which is useful if you have a separate procedure to update images (e.g. rollout) |
| metrics.destination | string | `"dev"` | The upstream sink for metrics. Supported values are "dev" and "prod" |
| migrations.enable_vfn_explicit_pvc | bool | `false` | See templates/fullnode.yaml |
| multicluster | object | `{"enabled":false,"targetClusters":["forge-multiregion-1","forge-multiregion-2","forge-multiregion-3"]}` | Options for multicluster mode. This is *experimental only*. |
| numFullnodeGroups | int | `1` | Total number of fullnode groups to deploy |
| numValidators | int | `1` | Number of validators to deploy |
| overrideNodeConfig | bool | `false` | Specify validator and fullnode NodeConfigs via named ConfigMaps, rather than the generated ones from this chart. |
| service.domain | string | `nil` | If set, the base domain name to use for External DNS |
| service.fullnode.enableAdminPort | bool | `false` | Enable the admin port on fullnodes |
| service.fullnode.enableMetricsPort | bool | `false` | Enable the metrics port on fullnodes |
| service.fullnode.enableRestApi | bool | `true` | Enable the REST API on fullnodes |
| service.fullnode.external.type | string | `"LoadBalancer"` | The Kubernetes ServiceType to use for fullnodes' HAProxy |
| service.fullnode.externalTrafficPolicy | string | `"Local"` | The externalTrafficPolicy for the fullnode service |
| service.fullnode.internal.annotations | object | `{}` |  |
| service.fullnode.internal.headless | bool | `false` |  |
| service.fullnode.internal.type | string | `"ClusterIP"` | The Kubernetes ServiceType to use for fullnodes |
| service.fullnode.loadBalancerSourceRanges | string | `nil` | If set and if the ServiceType is LoadBalancer, allow traffic to fullnodes from these CIDRs |
| service.internalDomain | string | `nil` | If set, the base domain name to use for internal LBs |
| service.validator.enableAdminPort | bool | `false` | Enable the admin port on the validator |
| service.validator.enableMetricsPort | bool | `false` | Enable the metrics port on the validator |
| service.validator.enableRestApi | bool | `true` | Enable the REST API on the validator |
| service.validator.external.type | string | `"LoadBalancer"` | The Kubernetes ServiceType to use for validator's HAProxy |
| service.validator.externalTrafficPolicy | string | `"Local"` | The externalTrafficPolicy for the validator service |
| service.validator.internal.annotations | object | `{}` |  |
| service.validator.internal.headless | bool | `false` |  |
| service.validator.internal.type | string | `"ClusterIP"` | The Kubernetes ServiceType to use for validator |
| service.validator.loadBalancerSourceRanges | string | `nil` | If set and if the ServiceType is LoadBalancer, allow traffic to validators from these CIDRs |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `nil` | The name of the service account to use. If not set and create is true, a name is generated using the fullname template |
| validator.affinity | object | `{}` |  |
| validator.config | object | `{}` | Validator configuration. See NodeConfig https://github.com/velor-chain/velor-core/blob/main/config/src/config/mod.rs |
| validator.enableNetworkPolicy | bool | `false` | Lock down network ingress and egress with Kubernetes NetworkPolicy |
| validator.force_enable_telemetry | bool | `false` | Flag to force enable telemetry service (useful for forge tests) |
| validator.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for validator images |
| validator.image.repo | string | `"velorlabs/validator"` | Image repo to use for validator images |
| validator.image.tag | string | `nil` | Image tag to use for validator images. If set, overrides `imageTag` |
| validator.name | string | `nil` | Internal: name of your validator for use in labels |
| validator.nodeSelector | object | `{}` |  |
| validator.podAnnotations | string | `nil` |  |
| validator.resources.limits.cpu | int | `30` |  |
| validator.resources.limits.memory | string | `"60Gi"` |  |
| validator.resources.requests.cpu | int | `30` |  |
| validator.resources.requests.memory | string | `"60Gi"` |  |
| validator.rust_log | string | `"info"` | Log level for the validator |
| validator.storage.class | string | `nil` | Kubernetes storage class to use for validator persistent storage |
| validator.storage.size | string | `"2048Gi"` | Size of validator persistent storage |
| validator.tolerations | list | `[]` |  |
| validator.useConsensusHealthCheckAsStartupProbe | bool | `false` |  |

## Resource Descriptions

Below is a list of the Kubernetes resources created by this helm chart.

The resources created by this helm chart will be prefixed with the helm release name. Below, they are denoted by
the `<RELEASE_NAME>` prefix.

StatefulSets:
* `<RELEASE_NAME>-velor-node-0-validator` - The validator StatefulSet
* `<RELEASE_NAME>-velor-node-0-fullnode-e<ERA>` - The fullnode StatefulSet

Deployments:
* `<RELEASE_NAME>-velor-node-0-validator` - The HAProxy deployment

PersistentVolumeClaim:
* `<RELEASE_NAME>-0-validator-e<ERA>` - The validator PersistentVolumeClaim
* `fn-<RELEASE_NAME>-0-fullnode-e<ERA>-0` - The fullnode PersistentVolumeClaim. Note the difference in naming scheme between valdiator and fullnode PVC, which is due to the fact that you can spin up multiple fullnodes, but only a single validator.

Services:
* `<RELEASE_NAME>-velor-node-0-validator-lb` - Inbound load balancer service that routes to the validator
* `<RELEASE_NAME>-velor-node-0-fullnode-lb` - Inbound load balancer service that routes to the fullnode

ConfigMaps:
* `<RELEASE_NAME>-0` - The validator and fullnode NodeConfigs
* `<RELEASE_NAME>-0-haproxy` - The HAProxy configuration

NetworkPolicies:
* `<RELEASE_NAME>-0-validator` - The validator NetworkPolicy, which controls network access to the validator pods

ServiceAccounts:
* [optional] `<RELEASE_NAME>` - The default service account
* `<RELEASE_NAME>-validator` - The validator service account
* `<RELEASE_NAME>-fullnode` - The fullnode service account

[optional] PodSecurityPolicy:
* `<RELEASE_NAME>` - The default PodSecurityPolicy for validators and fullnodes
* `<RELEASE_NAME>-haproxy` - The PodSecurityPolicy for HAProxy

## Common Operations

### Check Pod Status

```
$ kubectl get pods
```

You should see at least `1/1` replicas running for the validator, fullnode, and HAProxy. If there are any restarts, you should see it in this view.

To see more details about a singular pod, you can describe it:

```
$ kubectl describe pod <POD_NAME>
```

### Check the Pod Logs

```
$ kubectl logs <POD_NAME>
```

### Check all services

```
$ kubectl get services
```

By default, the services are `LoadBalancer` type, which means that they will be accessible from the outside world. Depending on your kubernetes deployment/cloud, the public IP or DNS information will be displayed.

### Scale Down Workloads

If you want to temporarily remove some of the workloads, you can scale them down.
```
# scale down the validator
kubectl scale statefulset <STS_NAME> --replicas=0
```

## Advanced Options

### Testnet Mode (Multiple Validators and Fullnodes)

For testing purposes, you may deploy multiple validators into the same cluster via `.Values.numValidators`. The naming convention is `<RELEASE_NAME>-velor-node-<INDEX>-validator`, where `<INDEX>` is the index of the validator. Note that for each validator, you must provide genesis ConfigMaps for each, of the name: `<RELEASE_NAME>-<INDEX>-genesis-e<ERA>`.
You may also deploy multiple fullnodes into the cluster via `.Values.numFullnodeGroups` and `.Values.fullnode.groups`. Each validator can have multiple fullnode groups, each with multiple replicas. The total number of fullnode groups can be limited via `.Values.numFullnodeGroups`.

### Era

The `.Values.chain.era` is a number that is incremented every time the validator's storage is wiped. This is useful for testnets when the network is wiped.

### Privileged Mode

For debugging purposes, it's sometimes useful to run the validator as root (privileged mode). This is enabled by `.Values.enablePrivilegedMode`.
