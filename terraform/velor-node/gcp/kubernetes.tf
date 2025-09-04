provider "kubernetes" {
  host                   = "https://${google_container_cluster.velor.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.velor.master_auth[0].cluster_ca_certificate)
  token                  = data.google_client_config.provider.access_token
}

resource "kubernetes_storage_class" "ssd" {
  metadata {
    name = "ssd"
  }
  storage_provisioner = "kubernetes.io/gce-pd"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "pd-ssd"
  }
}

provider "helm" {
  kubernetes {
    host                   = "https://${google_container_cluster.velor.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.velor.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

locals {
  # helm chart paths
  velor_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/velor-node"
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"

  # override the helm release name if an override exists, otherwise adopt the workspace name
  helm_release_name = var.helm_release_name_override != "" ? var.helm_release_name_override : local.workspace_name
}

resource "helm_release" "validator" {
  name        = local.helm_release_name
  chart       = local.velor_node_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      numValidators     = var.num_validators
      numFullnodeGroups = var.num_fullnode_groups
      imageTag          = var.image_tag
      manageImages      = var.manage_via_tf # if we're managing the entire deployment via terraform, override the images as well
      chain = {
        era      = var.era
        chain_id = var.chain_id
        name     = var.chain_name
      }
      validator = {
        name = var.validator_name
        config = {
          storage = {
            rocksdb_configs = {
              enable_storage_sharding = var.enable_storage_sharding
            }
          }
        }
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
          size  = var.validator_storage_size
        }
        nodeSelector = var.validator_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "validators"
        } : {}
        tolerations = [{
          key    = "velor.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      fullnode = {
        config = {
          storage = {
            rocksdb_configs = {
              enable_storage_sharding = var.enable_storage_sharding
            }
          }
        }
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
          size  = var.validator_storage_size
        }
        nodeSelector = var.validator_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "validators"
        } : {}
        tolerations = [{
          key    = "velor.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      haproxy = {
        nodeSelector = var.utility_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "utilities"
        } : {}
        tolerations = [{
          key    = "velor.org/nodepool"
          value  = "utilities"
          effect = "NoExecute"
        }]
      }
      service = {
        domain = local.domain
      }
    }),
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.velor_node_helm_chart_path, "**") : filesha1("${local.velor_node_helm_chart_path}/${f}")]))
    }
  }
}

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "${local.helm_release_name}-mon"
  chart       = local.monitoring_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      chain = {
        name = var.chain_name
      }
      validator = {
        name = var.validator_name
      }
      service = {
        domain = local.domain
      }
      monitoring = {
        prometheus = {
          storage = {
            class = kubernetes_storage_class.ssd.metadata[0].name
          }
        }
      }
      kube-state-metrics = {
        enabled = var.enable_kube_state_metrics
      }
      prometheus-node-exporter = {
        enabled = var.enable_prometheus_node_exporter
      }
    }),
    jsonencode(var.monitoring_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.monitoring_helm_chart_path, "**") : filesha1("${local.monitoring_helm_chart_path}/${f}")]))
  }
}
