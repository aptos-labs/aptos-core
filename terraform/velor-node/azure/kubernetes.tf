provider "kubernetes" {
  host                   = azurerm_kubernetes_cluster.velor.kube_config[0].host
  client_key             = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].client_key)
  client_certificate     = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].client_certificate)
  cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].cluster_ca_certificate)
}

provider "helm" {
  kubernetes {
    host                   = azurerm_kubernetes_cluster.velor.kube_config[0].host
    client_key             = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].client_key)
    client_certificate     = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].client_certificate)
    cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.velor.kube_config[0].cluster_ca_certificate)
  }
}

locals {
  # helm chart paths
  velor_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/velor-node"
}

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = local.velor_node_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      imageTag = var.image_tag
      chain = {
        era      = var.era
        chain_id = var.chain_id
        name     = var.chain_name
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
          key    = "velor.org/nodepool"
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
          key    = "velor.org/nodepool"
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
    value = sha1(join("", [for f in fileset(local.velor_node_helm_chart_path, "**") : filesha1("${local.velor_node_helm_chart_path}/${f}")]))
  }
}
