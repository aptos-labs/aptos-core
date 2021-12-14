variable "ssh_pub_key" {
  description = "SSH public key to configure for bastion and vault access"
}

resource "tls_private_key" "ca-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_self_signed_cert" "ca" {
  key_algorithm         = "ECDSA"
  private_key_pem       = tls_private_key.ca-key.private_key_pem
  validity_period_hours = 10 * 365 * 24
  early_renewal_hours   = 1 * 365 * 24
  is_ca_certificate     = true
  allowed_uses          = ["cert_signing"]

  subject {
    common_name  = "Vault CA"
    organization = "diem-${terraform.workspace}"
  }
}

resource "local_file" "ca" {
  filename        = "${terraform.workspace}-vault.ca"
  content         = tls_self_signed_cert.ca.cert_pem
  file_permission = "0644"
}

resource "tls_private_key" "vault-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_cert_request" "vault" {
  key_algorithm   = tls_private_key.vault-key.algorithm
  private_key_pem = tls_private_key.vault-key.private_key_pem
  dns_names       = ["localhost"]
  ip_addresses    = [azurerm_lb.vault.private_ip_address, "127.0.0.1"]

  subject {
    common_name  = azurerm_lb.vault.private_ip_address
    organization = "diem-${terraform.workspace}"
  }
}

resource "tls_locally_signed_cert" "vault" {
  cert_request_pem      = tls_cert_request.vault.cert_request_pem
  ca_key_algorithm      = tls_private_key.ca-key.algorithm
  ca_private_key_pem    = tls_private_key.ca-key.private_key_pem
  ca_cert_pem           = tls_self_signed_cert.ca.cert_pem
  validity_period_hours = tls_self_signed_cert.ca.validity_period_hours
  early_renewal_hours   = tls_self_signed_cert.ca.early_renewal_hours
  allowed_uses          = ["server_auth"]
}

resource "random_string" "vault-storage" {
  length  = 4
  upper   = false
  special = false
}

locals {
  workspace_sanitised = substr(replace(lower(terraform.workspace), "/[^a-z0-9]/", ""), 0, 14)
}

resource "azurerm_storage_account" "vault_" {
  name                     = "vault${local.workspace_sanitised}${random_string.vault-storage.result}"
  resource_group_name      = azurerm_resource_group.diem.name
  location                 = azurerm_resource_group.diem.location
  account_kind             = "BlockBlobStorage"
  account_tier             = "Premium"
  account_replication_type = "LRS"

  network_rules {
    default_action             = var.ssh_sources_ipv4 == ["0.0.0.0/0"] ? "Allow" : "Deny"
    ip_rules                   = var.ssh_sources_ipv4 == ["0.0.0.0/0"] ? [] : var.ssh_sources_ipv4
    virtual_network_subnet_ids = [azurerm_subnet.other.id]
  }
}

resource "azurerm_storage_container" "vault_" {
  name                 = "vault"
  storage_account_name = azurerm_storage_account.vault_.name

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_key_vault" "vault_" {
  name                = "vault${local.workspace_sanitised}${random_string.vault-storage.result}"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  tenant_id           = data.azurerm_client_config.current.tenant_id
  sku_name            = "standard"

  network_acls {
    default_action             = "Deny"
    bypass                     = "None"
    ip_rules                   = var.ssh_sources_ipv4
    virtual_network_subnet_ids = [azurerm_subnet.other.id]
  }
}

resource "azurerm_key_vault_access_policy" "terraform_" {
  key_vault_id       = azurerm_key_vault.vault_.id
  tenant_id          = data.azurerm_client_config.current.tenant_id
  object_id          = coalesce(var.key_vault_owner_id, data.azurerm_client_config.current.object_id)
  key_permissions    = ["get", "list", "create", "delete", "update", "backup", "restore"]
  secret_permissions = ["get", "list", "set", "delete"]
}

resource "azurerm_key_vault_access_policy" "vault" {
  key_vault_id       = azurerm_key_vault.vault_.id
  tenant_id          = data.azurerm_client_config.current.tenant_id
  object_id          = azurerm_user_assigned_identity.vault.principal_id
  key_permissions    = ["get", "wrapKey", "unwrapKey"]
  secret_permissions = ["get"]
}

resource "azurerm_key_vault_key" "vault_" {
  name         = "vault"
  key_vault_id = azurerm_key_vault.vault_.id
  key_type     = "RSA"
  key_size     = 2048
  key_opts     = ["decrypt", "encrypt", "sign", "verify", "wrapKey", "unwrapKey"]

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_key_vault_secret" "vault-tls" {
  name         = "vault-tls"
  key_vault_id = azurerm_key_vault.vault_.id
  value        = tls_private_key.vault-key.private_key_pem
}

resource "azurerm_public_ip" "bastion" {
  name                = "diem-${terraform.workspace}-bastion"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_network_interface" "bastion" {
  name                = "diem-${terraform.workspace}-bastion"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location

  ip_configuration {
    name                          = "internal"
    primary                       = true
    subnet_id                     = azurerm_subnet.other.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.bastion.id
  }
}

resource "azurerm_network_interface_application_security_group_association" "bastion" {
  network_interface_id          = azurerm_network_interface.bastion.id
  application_security_group_id = azurerm_application_security_group.bastion.id
}

resource "azurerm_linux_virtual_machine" "bastion" {
  count                 = var.bastion_enable ? 1 : 0
  name                  = "diem-${terraform.workspace}-bastion"
  resource_group_name   = azurerm_resource_group.diem.name
  location              = azurerm_resource_group.diem.location
  size                  = "Standard_B1LS"
  admin_username        = "az-user"
  network_interface_ids = [azurerm_network_interface.bastion.id]
  custom_data           = base64encode(file("${path.module}/templates/bastion_user_data.cloud"))

  admin_ssh_key {
    username   = "az-user"
    public_key = var.ssh_pub_key
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "UbuntuServer"
    sku       = "18.04-LTS"
    version   = "latest"
  }

  os_disk {
    storage_account_type = "Standard_LRS"
    caching              = "None"
  }
}

data "template_file" "vault_user_data" {
  template = file("${path.module}/templates/vault_user_data.sh")

  vars = {
    vault_version    = "1.8.1"
    vault_sha256     = "bb411f2bbad79c2e4f0640f1d3d5ef50e2bda7d4f40875a56917c95ff783c2db"
    vault_ca         = tls_self_signed_cert.ca.cert_pem
    vault_cert       = tls_locally_signed_cert.vault.cert_pem
    vault_key_vault  = azurerm_key_vault.vault_.name
    vault_key_secret = azurerm_key_vault_secret.vault-tls.name
    vault_config = jsonencode({
      cluster_addr = "https://$LOCAL_IPV4:8201"
      api_addr     = "https://${azurerm_lb.vault.private_ip_address}:8200"
      storage = {
        azure = {
          accountName = azurerm_storage_account.vault_.name
          accountKey  = azurerm_storage_account.vault_.primary_access_key
          container   = azurerm_storage_container.vault_.name
        }
      }
      listener = {
        tcp = {
          address       = "[::]:8200"
          tls_cert_file = "/etc/vault/vault.crt"
          tls_key_file  = "/etc/vault/vault.key"
          telemetry = {
            unauthenticated_metrics_access = true
          }
        }
      }
      seal = {
        azurekeyvault = {
          tenant_id  = azurerm_key_vault.vault_.tenant_id
          vault_name = azurerm_key_vault.vault_.name
          key_name   = azurerm_key_vault_key.vault_.name
        }
      }
      telemetry = {
        disable_hostname = true
      }
    })
  }
}

resource "azurerm_linux_virtual_machine_scale_set" "vault" {
  name                = "diem-${terraform.workspace}-vault"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  sku                 = "Standard_F2s_v2"
  instances           = var.vault_num
  admin_username      = "az-user"
  custom_data         = base64encode(data.template_file.vault_user_data.rendered)

  admin_ssh_key {
    username   = "az-user"
    public_key = var.ssh_pub_key
  }

  identity {
    type         = "UserAssigned"
    identity_ids = [azurerm_user_assigned_identity.vault.id]
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "UbuntuServer"
    sku       = "18.04-LTS"
    version   = "latest"
  }

  os_disk {
    storage_account_type = "Standard_LRS"
    caching              = "None"
  }

  network_interface {
    name    = "internal"
    primary = true

    ip_configuration {
      name                                   = "internal"
      primary                                = true
      subnet_id                              = azurerm_subnet.other.id
      application_security_group_ids         = [azurerm_application_security_group.vault.id]
      load_balancer_backend_address_pool_ids = [azurerm_lb_backend_address_pool.vault.id]
    }
  }
}

resource "azurerm_lb" "vault" {
  name                = "diem-${terraform.workspace}-vault"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  sku                 = "Standard"

  frontend_ip_configuration {
    name                          = "internal"
    subnet_id                     = azurerm_subnet.other.id
    private_ip_address_allocation = "Dynamic"
  }
}

resource "azurerm_lb_backend_address_pool" "vault" {
  name                = "vault"
  loadbalancer_id     = azurerm_lb.vault.id
}

resource "azurerm_lb_probe" "vault" {
  name                = "vault-active"
  resource_group_name = azurerm_resource_group.diem.name
  loadbalancer_id     = azurerm_lb.vault.id
  protocol            = "Https"
  port                = 8200
  request_path        = "/v1/sys/health"
}

resource "azurerm_lb_rule" "vault" {
  name                           = "vault"
  resource_group_name            = azurerm_resource_group.diem.name
  loadbalancer_id                = azurerm_lb.vault.id
  protocol                       = "Tcp"
  frontend_port                  = 8200
  backend_port                   = 8200
  frontend_ip_configuration_name = "internal"
  backend_address_pool_id        = azurerm_lb_backend_address_pool.vault.id
  probe_id                       = azurerm_lb_probe.vault.id
}
