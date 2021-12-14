provider "kubernetes" {
  host                   = azurerm_kubernetes_cluster.diem.kube_config[0].host
  client_key             = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].client_key)
  client_certificate     = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].client_certificate)
  cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].cluster_ca_certificate)
}

provider "helm" {
  kubernetes {
    host                   = azurerm_kubernetes_cluster.diem.kube_config[0].host
    client_key             = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].client_key)
    client_certificate     = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].client_certificate)
    cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].cluster_ca_certificate)
  }
}

locals {
  vault          = {}
  network_values = "${path.module}/../helm/values/${split("-", terraform.workspace)[0]}.yaml"
}

resource "helm_release" "validator" {
  name        = terraform.workspace
  chart       = var.helm_chart
  max_history = 100
  wait        = false

  values = [
    jsonencode({
      validator = {
        name = var.validator_name
        storage = {
          class = "managed-premium"
        }
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
      safetyrules = {
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.trusted.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.trusted.name
          effect = "NoExecute"
        }]
      }
      keymanager = {
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.trusted.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.trusted.name
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
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
      haproxy = {
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
      monitoring = {
        fullKubernetesScrape = true
        useKubeStateMetrics = var.use_kube_state_metrics
        prometheus = {
          storage = {
            class = "default"
          }
        }
      }
      backup = {
        config = {
          location = "azure"
          azure = {
            account   = azurerm_storage_account.backup.name
            container = azurerm_storage_container.backup.name
            sas       = data.azurerm_storage_account_blob_container_sas.backup.sas
          }
        }
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }
      restore = {
        config = {
          location = "azure"
          azure = {
            account   = azurerm_storage_account.backup.name
            container = azurerm_storage_container.backup.name
            sas       = data.azurerm_storage_account_blob_container_sas.backup.sas
          }
        }
        nodeSelector = {
          "agentpool" = azurerm_kubernetes_cluster_node_pool.validators.name
        }
        tolerations = [{
          key    = "diem.org/nodepool"
          value  = azurerm_kubernetes_cluster_node_pool.validators.name
          effect = "NoExecute"
        }]
      }

      vault = local.vault
    }),
    fileexists(local.network_values) ? file(local.network_values) : "{}",
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  set {
    name  = "timestamp"
    value = var.helm_force_update ? timestamp() : ""
  }
}

resource "helm_release" "kube-state-metrics" {
  count      = var.use_kube_state_metrics ? 1 : 0
  name       = "kube-state-metrics"
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "kube-state-metrics"
  version    = "3.4.1"
}

resource "local_file" "kubernetes" {
  filename = "${terraform.workspace}-kubernetes.json"
  content = jsonencode({
    kubernetes_host        = azurerm_kubernetes_cluster.diem.kube_config[0].host
    kubernetes_ca_cert     = base64decode(azurerm_kubernetes_cluster.diem.kube_config[0].cluster_ca_certificate)
    issuer                 = azurerm_kubernetes_cluster.diem.fqdn
    service_account_prefix = "${terraform.workspace}-diem-validator"
    pod_cidrs              = azurerm_subnet.nodes.address_prefixes
  })
  file_permission = "0644"
}
