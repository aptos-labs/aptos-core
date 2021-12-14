resource "random_string" "backup-storage" {
  length  = 4
  upper   = false
  special = false
}

resource "azurerm_storage_account" "backup" {
  name                     = "backup${local.workspace_sanitised}${random_string.backup-storage.result}"
  resource_group_name      = azurerm_resource_group.diem.name
  location                 = azurerm_resource_group.diem.location
  account_tier             = "Standard"
  account_replication_type = var.backup_replication_type
  allow_blob_public_access = var.backup_public_access

  network_rules {
    default_action             = (var.backup_public_access || var.ssh_sources_ipv4 == ["0.0.0.0/0"]) ? "Allow" : "Deny"
    ip_rules                   = (var.backup_public_access || var.ssh_sources_ipv4 == ["0.0.0.0/0"]) ? [] : var.ssh_sources_ipv4
    virtual_network_subnet_ids = [azurerm_subnet.nodes.id]
  }
}

resource "azurerm_storage_container" "backup" {
  name                  = "backup"
  storage_account_name  = azurerm_storage_account.backup.name
  container_access_type = var.backup_public_access ? "container" : "private"
}

resource "time_rotating" "sas" {
  rotation_years = 1
}

data "azurerm_storage_account_blob_container_sas" "backup" {
  connection_string = azurerm_storage_account.backup.primary_connection_string
  container_name    = azurerm_storage_container.backup.name

  start  = time_rotating.sas.id
  expiry = timeadd(time_rotating.sas.id, "17520h") # 2 years

  permissions {
    read   = true
    add    = true
    create = true
    write  = true
    list   = true
    delete = false
  }
}
