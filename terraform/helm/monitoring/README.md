# velor-monitoring

![Version: 0.2.0](https://img.shields.io/badge/Version-0.2.0-informational?style=flat-square)

## Requirements

| Repository | Name | Version |
|------------|------|---------|
| https://prometheus-community.github.io/helm-charts | kube-state-metrics | 4.16.0 |
| https://prometheus-community.github.io/helm-charts | prometheus-node-exporter | 4.0.0 |

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| chain.name | string | `nil` |  |
| fullnode.name | string | `nil` |  |
| kube-state-metrics.enabled | bool | `false` |  |
| kube-state-metrics.namespaceOverride | string | `"kube-system"` |  |
| kube-state-metrics.podAnnotations."prometheus.io/port" | string | `"8080"` |  |
| kube-state-metrics.podAnnotations."prometheus.io/scrape" | string | `"true"` |  |
| monitoring.affinity | object | `{}` |  |
| monitoring.alertmanager.alertReceivers[0].name | string | `"critical"` |  |
| monitoring.alertmanager.alertReceivers[1].name | string | `"error"` |  |
| monitoring.alertmanager.alertReceivers[2].name | string | `"default"` |  |
| monitoring.alertmanager.alertRouteTrees[0].match.severity | string | `"critical"` |  |
| monitoring.alertmanager.alertRouteTrees[0].receiver | string | `"critical"` |  |
| monitoring.alertmanager.alertRouteTrees[1].match.severity | string | `"error"` |  |
| monitoring.alertmanager.alertRouteTrees[1].receiver | string | `"error"` |  |
| monitoring.alertmanager.image.pullPolicy | string | `"IfNotPresent"` |  |
| monitoring.alertmanager.image.repo | string | `"prom/alertmanager"` |  |
| monitoring.alertmanager.image.tag | string | `"v0.24.0@sha256:b1ba90841a82ea24d79d4e6255b96025a9e89275bec0fae87d75a5959461971e"` |  |
| monitoring.alertmanager.resources.limits.cpu | float | `0.1` |  |
| monitoring.alertmanager.resources.limits.memory | string | `"128Mi"` |  |
| monitoring.alertmanager.resources.requests.cpu | float | `0.1` |  |
| monitoring.alertmanager.resources.requests.memory | string | `"128Mi"` |  |
| monitoring.grafana.config | string | `nil` |  |
| monitoring.grafana.env.GF_AUTH_ANONYMOUS_ENABLED | bool | `true` |  |
| monitoring.grafana.env.GF_AUTH_ANONYMOUS_ORG_ROLE | string | `"Editor"` |  |
| monitoring.grafana.googleAuth | string | `nil` |  |
| monitoring.grafana.image.pullPolicy | string | `"IfNotPresent"` |  |
| monitoring.grafana.image.repo | string | `"grafana/grafana"` |  |
| monitoring.grafana.image.tag | string | `"9.0.9@sha256:4a6b9d8d88522d2851f947f8f84cca10b6a43ca26d5e93102daf3a87935f10a5"` |  |
| monitoring.grafana.resources.limits.cpu | int | `1` |  |
| monitoring.grafana.resources.limits.memory | string | `"256Mi"` |  |
| monitoring.grafana.resources.requests.cpu | int | `1` |  |
| monitoring.grafana.resources.requests.memory | string | `"256Mi"` |  |
| monitoring.nodeSelector | object | `{}` |  |
| monitoring.prometheus.deleteWal | bool | `false` |  |
| monitoring.prometheus.fullKubernetesScrape | bool | `false` |  |
| monitoring.prometheus.image.pullPolicy | string | `"IfNotPresent"` |  |
| monitoring.prometheus.image.repo | string | `"prom/prometheus"` |  |
| monitoring.prometheus.image.tag | string | `"v2.34.0@sha256:cb42332b66ac51a05c52f255e48a4496c0a172676093123bf28b37762009e78a"` |  |
| monitoring.prometheus.remote_write.enabled | bool | `false` |  |
| monitoring.prometheus.remote_write.region | string | `nil` |  |
| monitoring.prometheus.remote_write.url | string | `nil` |  |
| monitoring.prometheus.resources.limits.cpu | int | `1` |  |
| monitoring.prometheus.resources.limits.memory | string | `"1.5Gi"` |  |
| monitoring.prometheus.resources.requests.cpu | int | `1` |  |
| monitoring.prometheus.resources.requests.memory | string | `"1.5Gi"` |  |
| monitoring.prometheus.storage.class | string | `nil` |  |
| monitoring.prometheus.storage.size | string | `"100Gi"` |  |
| monitoring.prometheus.tsdb_max_block_duration | string | `"1h"` |  |
| monitoring.prometheus.tsdb_min_block_duration | string | `"30m"` |  |
| monitoring.prometheus.tsdb_retention_time | string | `"15d"` |  |
| monitoring.pushgateway.image.pullPolicy | string | `"IfNotPresent"` |  |
| monitoring.pushgateway.image.repo | string | `"prom/pushgateway"` |  |
| monitoring.pushgateway.image.tag | string | `"v1.4.1@sha256:b561435cb17ee816c5d90c2408bcc1ffe25304f1608e18db16a3969f6cc44626"` |  |
| monitoring.pushgateway.resources.limits.cpu | float | `0.1` |  |
| monitoring.pushgateway.resources.limits.memory | string | `"128Mi"` |  |
| monitoring.pushgateway.resources.requests.cpu | float | `0.1` |  |
| monitoring.pushgateway.resources.requests.memory | string | `"128Mi"` |  |
| monitoring.serviceAccount.annotations | object | `{}` |  |
| monitoring.tolerations | list | `[]` |  |
| prometheus-node-exporter.enabled | bool | `false` |  |
| prometheus-node-exporter.namespaceOverride | string | `"kube-system"` |  |
| prometheus-node-exporter.podAnnotations."prometheus.io/port" | string | `"9100"` |  |
| prometheus-node-exporter.podAnnotations."prometheus.io/scrape" | string | `"true"` |  |
| service.domain | string | `nil` |  |
| service.external.type | string | `"LoadBalancer"` |  |
| service.monitoring.loadBalancerSourceRanges | string | `nil` |  |
| serviceAccount.annotations | string | `nil` |  |
| serviceAccount.create | bool | `true` |  |
| serviceAccount.name | string | `nil` |  |
| validator.name | string | `nil` |  |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
