locals {
  vnet_address = "192.168.0.0/16"
}

resource "azurerm_virtual_network" "diem" {
  name                = "diem-${terraform.workspace}"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  address_space       = [local.vnet_address]
}

resource "azurerm_subnet" "nodes" {
  name                 = "nodes"
  resource_group_name  = azurerm_resource_group.diem.name
  virtual_network_name = azurerm_virtual_network.diem.name
  address_prefixes     = [cidrsubnet(local.vnet_address, 4, 0)]
  service_endpoints    = ["Microsoft.Storage"]
}

resource "azurerm_subnet" "other" {
  name                 = "other"
  resource_group_name  = azurerm_resource_group.diem.name
  virtual_network_name = azurerm_virtual_network.diem.name
  address_prefixes     = [cidrsubnet(local.vnet_address, 4, 1)]
  service_endpoints    = ["Microsoft.KeyVault", "Microsoft.Storage"]
}

resource "azurerm_public_ip" "nat" {
  name                = "diem-${terraform.workspace}-nat"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_nat_gateway" "nat" {
  name                  = "diem-${terraform.workspace}-nat"
  resource_group_name   = azurerm_resource_group.diem.name
  location              = azurerm_resource_group.diem.location
}

resource "azurerm_nat_gateway_public_ip_association" "nat" {
  nat_gateway_id       = azurerm_nat_gateway.nat.id
  public_ip_address_id = azurerm_public_ip.nat.id
}

resource "azurerm_subnet_nat_gateway_association" "nat" {
  subnet_id      = azurerm_subnet.other.id
  nat_gateway_id = azurerm_nat_gateway.nat.id
}

resource "azurerm_application_security_group" "bastion" {
  name                = "diem-${terraform.workspace}-bastion"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
}

resource "azurerm_application_security_group" "vault" {
  name                = "diem-${terraform.workspace}-vault"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location
}

resource "azurerm_network_security_group" "other" {
  name                = "diem-${terraform.workspace}-other"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location

  security_rule {
    name                                       = "bastion-ssh"
    priority                                   = 1000
    direction                                  = "Inbound"
    access                                     = "Allow"
    protocol                                   = "Tcp"
    destination_application_security_group_ids = [azurerm_application_security_group.bastion.id]
    destination_port_range                     = "22"
    source_address_prefixes                    = var.ssh_sources_ipv4
    source_port_range                          = "*"
  }

  security_rule {
    name                                       = "vault-ssh"
    priority                                   = 1010
    direction                                  = "Inbound"
    access                                     = "Allow"
    protocol                                   = "Tcp"
    destination_application_security_group_ids = [azurerm_application_security_group.vault.id]
    destination_port_range                     = "22"
    source_application_security_group_ids      = [azurerm_application_security_group.bastion.id]
    source_port_range                          = "*"
  }

  security_rule {
    name                                       = "vault-replication"
    priority                                   = 1020
    direction                                  = "Inbound"
    access                                     = "Allow"
    protocol                                   = "Tcp"
    destination_application_security_group_ids = [azurerm_application_security_group.vault.id]
    destination_port_range                     = "8200-8201"
    source_application_security_group_ids      = [azurerm_application_security_group.vault.id]
    source_port_range                          = "*"
  }

  security_rule {
    name                                       = "vault-api"
    priority                                   = 1030
    direction                                  = "Inbound"
    access                                     = "Allow"
    protocol                                   = "Tcp"
    destination_application_security_group_ids = [azurerm_application_security_group.vault.id]
    destination_port_range                     = "8200"
    source_address_prefix                      = "VirtualNetwork"
    source_port_range                          = "*"
  }

  security_rule {
    name                       = "deny-all-inbound"
    priority                   = 4000
    direction                  = "Inbound"
    access                     = "Deny"
    protocol                   = "*"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefix      = "*"
    source_port_range          = "*"
  }

  security_rule {
    name                                  = "bastion-ssh-out"
    priority                              = 1000
    direction                             = "Outbound"
    access                                = "Allow"
    protocol                              = "Tcp"
    destination_address_prefix            = "VirtualNetwork"
    destination_port_range                = "22"
    source_application_security_group_ids = [azurerm_application_security_group.bastion.id]
    source_port_range                     = "*"
  }

  security_rule {
    name                                       = "vault-replication-out"
    priority                                   = 1010
    direction                                  = "Outbound"
    access                                     = "Allow"
    protocol                                   = "Tcp"
    destination_application_security_group_ids = [azurerm_application_security_group.vault.id]
    destination_port_range                     = "8200-8201"
    source_application_security_group_ids      = [azurerm_application_security_group.vault.id]
    source_port_range                          = "*"
  }

  security_rule {
    name                                  = "vault-https"
    priority                              = 1020
    direction                             = "Outbound"
    access                                = "Allow"
    protocol                              = "Tcp"
    destination_address_prefix            = "*"
    destination_port_range                = "443"
    source_application_security_group_ids = [azurerm_application_security_group.vault.id]
    source_port_range                     = "*"
  }

  security_rule {
    name                                  = "vault-http"
    priority                              = 1030
    direction                             = "Outbound"
    access                                = "Allow"
    protocol                              = "Tcp"
    destination_address_prefix            = "*"
    destination_port_range                = "80"
    source_application_security_group_ids = [azurerm_application_security_group.vault.id]
    source_port_range                     = "*"
  }

  security_rule {
    name                       = "deny-all-outbound"
    priority                   = 4000
    direction                  = "Outbound"
    access                     = "Deny"
    protocol                   = "*"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefix      = "*"
    source_port_range          = "*"
  }
}

resource "azurerm_subnet_network_security_group_association" "other" {
  subnet_id                 = azurerm_subnet.other.id
  network_security_group_id = azurerm_network_security_group.other.id
}

locals {
  cluster_ips = concat(
    azurerm_subnet.nodes.address_prefixes,
    [azurerm_kubernetes_cluster.diem.network_profile[0].service_cidr,
    azurerm_kubernetes_cluster.diem.network_profile[0].pod_cidr]
  )
}

resource "azurerm_network_security_group" "nodes" {
  name                = "diem-${terraform.workspace}-nodes"
  resource_group_name = azurerm_resource_group.diem.name
  location            = azurerm_resource_group.diem.location

  security_rule {
    name                         = "nodes-tcp"
    priority                     = 1000
    direction                    = "Inbound"
    access                       = "Allow"
    protocol                     = "Tcp"
    destination_address_prefix   = "*"
    destination_port_range       = "1025-65535"
    source_address_prefixes      = local.cluster_ips
    source_port_range            = "*"
  }

  security_rule {
    name                         = "nodes-udp"
    priority                     = 1010
    direction                    = "Inbound"
    access                       = "Allow"
    protocol                     = "Udp"
    destination_address_prefix   = "*"
    destination_port_range       = "1025-65535"
    source_address_prefixes      = local.cluster_ips
    source_port_range            = "*"
  }

  security_rule {
    name                         = "nodes-icmp"
    priority                     = 1020
    direction                    = "Inbound"
    access                       = "Allow"
    protocol                     = "Icmp"
    destination_address_prefix   = "*"
    destination_port_range       = "*"
    source_address_prefixes      = local.cluster_ips
    source_port_range            = "*"
  }

  security_rule {
    name                         = "nodes-dns"
    priority                     = 1030
    direction                    = "Inbound"
    access                       = "Allow"
    protocol                     = "Udp"
    destination_address_prefix   = "*"
    destination_port_range       = "53"
    source_address_prefixes      = local.cluster_ips
    source_port_range            = "*"
  }

  security_rule {
    name                       = "allow-load-balancer-inbound"
    priority                   = 3000
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "*"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefix      = "AzureLoadBalancer"
    source_port_range          = "*"
  }

  # This allows all traffic from the Internet, but AKS applies a Network
  # Security Group to the interfaces of the instances which will only
  # allow connections to LoadBalancer Kubernetes services.
  security_rule {
    name                       = "allow-internet-inbound"
    priority                   = 3010
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "*"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefix      = "Internet"
    source_port_range          = "*"
  }

  security_rule {
    name                       = "deny-all-inbound"
    priority                   = 4000
    direction                  = "Inbound"
    access                     = "Deny"
    protocol                   = "*"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefix      = "*"
    source_port_range          = "*"
  }
}

resource "azurerm_subnet_network_security_group_association" "nodes" {
  subnet_id                 = azurerm_subnet.nodes.id
  network_security_group_id = azurerm_network_security_group.nodes.id
}
