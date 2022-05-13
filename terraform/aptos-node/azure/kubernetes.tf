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

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/aptos-node"
  max_history = 100
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
          class = "default"
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

  set {
    name  = "timestamp"
    value = var.helm_force_update ? timestamp() : ""
  }
}

resource "helm_release" "logger" {
  count       = var.enable_logger ? 1 : 0
  name        = "${terraform.workspace}-log"
  chart       = "${path.module}/../../helm/logger"
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
        name = "${terraform.workspace}-aptos-node-validator"
      }
    }),
    jsonencode(var.logger_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "${terraform.workspace}-mon"
  chart       = "${path.module}/../../helm/monitoring"
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

  set {
    name  = "timestamp"
    value = timestamp()
  }
}
