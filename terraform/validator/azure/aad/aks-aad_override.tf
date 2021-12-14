resource "azurerm_kubernetes_cluster" "diem" {
  role_based_access_control {
    enabled = true
    azure_active_directory {
      managed                = true
      admin_group_object_ids = var.k8s_admin_groups
      tenant_id              = data.azurerm_client_config.current.tenant_id
    }
  }
}

provider "kubernetes" {
  host                   = azurerm_kubernetes_cluster.diem.kube_admin_config[0].host
  client_key             = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].client_key)
  client_certificate     = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].client_certificate)
  cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].cluster_ca_certificate)
}

provider "helm" {
  kubernetes {
    host                   = azurerm_kubernetes_cluster.diem.kube_admin_config[0].host
    client_key             = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].client_key)
    client_certificate     = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].client_certificate)
    cluster_ca_certificate = base64decode(azurerm_kubernetes_cluster.diem.kube_admin_config[0].cluster_ca_certificate)
  }
}
