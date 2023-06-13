provider "kubernetes" {
  host                   = "https://${google_container_cluster.aptos.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
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
    host                   = "https://${google_container_cluster.aptos.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

locals {
  # helm chart paths
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"
  logger_helm_chart_path     = "${path.module}/../../helm/logger"
  aptos_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/aptos-node"

  # override the helm release name if an override exists, otherwise adopt the workspace name
  helm_release_name = var.helm_release_name_override != "" ? var.helm_release_name_override : local.workspace_name
}

resource "helm_release" "validator" {
  name        = local.helm_release_name
  chart       = local.aptos_node_helm_chart_path
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
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
        }
        nodeSelector = var.gke_enable_node_autoprovisioning ? {} : {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.validators.name
        }
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      fullnode = {
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
        }
        nodeSelector = var.gke_enable_node_autoprovisioning ? {} : {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.validators.name
        }
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      haproxy = {
        nodeSelector = var.gke_enable_node_autoprovisioning ? {} : {
          "cloud.google.com/gke-nodepool" = google_container_node_pool.utilities.name
        }
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
      value = sha1(join("", [for f in fileset(local.aptos_node_helm_chart_path, "**") : filesha1("${local.aptos_node_helm_chart_path}/${f}")]))
    }
  }
}

resource "helm_release" "logger" {
  count       = var.enable_logger ? 1 : 0
  name        = "${local.helm_release_name}-log"
  chart       = local.logger_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      logger = {
        name = "aptos-logger"
      }
      chain = {
        name = var.chain_name
      }
      serviceAccount = {
        create = false
        # this name must match the serviceaccount created by the aptos-node helm chart
      name = local.helm_release_name == "aptos-node" ? "aptos-node-validator" : "${local.helm_release_name}-aptos-node-validator" }
    }),
    jsonencode(var.logger_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.logger_helm_chart_path, "**") : filesha1("${local.logger_helm_chart_path}/${f}")]))
  }
}

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "${local.helm_release_name}-mon"
  chart       = local.monitoring_helm_chart_path
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      chain = {
        name = var.chain_name
      }
      validator = {
        name = var.validator_name
      }
      monitoring = {
        prometheus = {
          storage = {
            class = kubernetes_storage_class.ssd.metadata[0].name
          }
        }
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

resource "helm_release" "node_exporter" {
  count       = var.enable_node_exporter ? 1 : 0
  name        = "prometheus-node-exporter"
  repository  = "https://prometheus-community.github.io/helm-charts"
  chart       = "prometheus-node-exporter"
  version     = "4.0.0"
  namespace   = "kube-system"
  max_history = 5
  wait        = false

  values = [
    jsonencode({}),
    jsonencode(var.node_exporter_helm_values),
  ]
}
