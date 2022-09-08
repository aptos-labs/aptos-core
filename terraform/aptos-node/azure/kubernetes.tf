provider "kubernetes" {
  host                   = azurerm_kubernetes_cluster.aptos.kube_config[0].host
  client_key             = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].client_key)
  client_certificate     = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].client_certificate)
  cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].cluster_ca_certificate)
}

provider "helm" {
  kubernetes {
    host                   = azurerm_kubernetes_cluster.aptos.kube_config[0].host
    client_key             = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].client_key)
    client_certificate     = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].client_certificate)
    cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.aptos.kube_config[0].cluster_ca_certificate)
  }
}

locals {
  # helm chart paths
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"
  logger_helm_chart_path     = "${path.module}/../../helm/logger"
  aptos_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/aptos-node"
}

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = local.aptos_node_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      imageTag = var.image_tag
      chain = {
        era        = var.era
        chain_id   = var.chain_id
        chain_name = var.chain_name
      }
      validator = {
        name = var.validator_name
        storage = {
          class = "managed-premium"
        }
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
      fullnode = {
        storage = {
          class = "managed-premium"
        }
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
    }),
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.aptos_node_helm_chart_path, "**") : filesha1("${local.aptos_node_helm_chart_path}/${f}")]))
  }
}

resource "helm_release" "logger" {
  count       = var.enable_logger ? 1 : 0
  name        = "${terraform.workspace}-log"
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
        name   = "${terraform.workspace}-aptos-node-validator"
      }
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
  name        = "${terraform.workspace}-mon"
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
            class = "default"
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
