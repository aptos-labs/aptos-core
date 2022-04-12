locals {
  kubernetes_version = "1.21.2"
}

resource "azurerm_kubernetes_cluster" "aptos" {
  name                            = "aptos-${terraform.workspace}"
  resource_group_name             = azurerm_resource_group.aptos.name
  location                        = azurerm_resource_group.aptos.location
  dns_prefix                      = "aptos-${terraform.workspace}"
  kubernetes_version              = local.kubernetes_version
  api_server_authorized_ip_ranges = concat(["${azurerm_public_ip.nat.ip_address}/32"], var.k8s_api_sources)

  role_based_access_control {
    enabled = true
  }

  network_profile {
    network_plugin    = "kubenet"
    network_policy    = "calico"
    load_balancer_sku = "standard"
  }

  default_node_pool {
    name                 = "utilities"
    orchestrator_version = local.kubernetes_version
    vm_size              = "Standard_B2s"
    vnet_subnet_id       = azurerm_subnet.nodes.id
    node_count           = lookup(var.node_pool_sizes, "utilities", 3)
    os_disk_size_gb      = 30
  }

  service_principal {
    client_id     = azuread_service_principal.aptos.application_id
    client_secret = azuread_application_password.aptos.value
  }
}

resource "azurerm_kubernetes_cluster_node_pool" "validators" {
  kubernetes_cluster_id = azurerm_kubernetes_cluster.aptos.id
  orchestrator_version  = azurerm_kubernetes_cluster.aptos.kubernetes_version

  name            = "validators"
  vm_size         = "Standard_F4s_v2"
  vnet_subnet_id  = azurerm_subnet.nodes.id
  node_count      = lookup(var.node_pool_sizes, "validators", 3)
  os_disk_size_gb = 30
  node_taints     = ["aptos.org/nodepool=validators:NoExecute"]
}

resource "azurerm_log_analytics_workspace" "aptos" {
  name                = "aptos-${terraform.workspace}"
  resource_group_name = azurerm_resource_group.aptos.name
  location            = azurerm_resource_group.aptos.location
  retention_in_days   = 30
}

resource "azurerm_monitor_diagnostic_setting" "cluster" {
  name                       = "cluster"
  target_resource_id         = azurerm_kubernetes_cluster.aptos.id
  log_analytics_workspace_id = azurerm_log_analytics_workspace.aptos.id

  log { category = "kube-apiserver" }
  log { category = "kube-controller-manager" }
  log { category = "kube-scheduler" }
  log { category = "kube-audit" }
  log { category = "guard" }
}
