locals {
  vnet_address = "192.168.0.0/16"
}

resource "azurerm_virtual_network" "velor" {
  name                = "velor-${terraform.workspace}"
  resource_group_name = azurerm_resource_group.velor.name
  location            = azurerm_resource_group.velor.location
  address_space       = [local.vnet_address]
}

resource "azurerm_subnet" "nodes" {
  name                 = "nodes"
  resource_group_name  = azurerm_resource_group.velor.name
  virtual_network_name = azurerm_virtual_network.velor.name
  address_prefixes     = [cidrsubnet(local.vnet_address, 4, 0)]
  service_endpoints    = ["Microsoft.Storage"]
}

resource "azurerm_public_ip" "nat" {
  name                = "velor-${terraform.workspace}-nat"
  resource_group_name = azurerm_resource_group.velor.name
  location            = azurerm_resource_group.velor.location
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_nat_gateway" "nat" {
  name                = "velor-${terraform.workspace}-nat"
  resource_group_name = azurerm_resource_group.velor.name
  location            = azurerm_resource_group.velor.location
}

resource "azurerm_nat_gateway_public_ip_association" "nat" {
  nat_gateway_id       = azurerm_nat_gateway.nat.id
  public_ip_address_id = azurerm_public_ip.nat.id
}

locals {
  cluster_ips = concat(
    azurerm_subnet.nodes.address_prefixes,
    [azurerm_kubernetes_cluster.velor.network_profile[0].service_cidr,
    azurerm_kubernetes_cluster.velor.network_profile[0].pod_cidr]
  )
}

resource "azurerm_network_security_group" "nodes" {
  name                = "velor-${terraform.workspace}-nodes"
  resource_group_name = azurerm_resource_group.velor.name
  location            = azurerm_resource_group.velor.location

  security_rule {
    name                       = "nodes-tcp"
    priority                   = 1000
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    destination_address_prefix = "*"
    destination_port_range     = "1025-65535"
    source_address_prefixes    = local.cluster_ips
    source_port_range          = "*"
  }

  security_rule {
    name                       = "nodes-udp"
    priority                   = 1010
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Udp"
    destination_address_prefix = "*"
    destination_port_range     = "1025-65535"
    source_address_prefixes    = local.cluster_ips
    source_port_range          = "*"
  }

  security_rule {
    name                       = "nodes-icmp"
    priority                   = 1020
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Icmp"
    destination_address_prefix = "*"
    destination_port_range     = "*"
    source_address_prefixes    = local.cluster_ips
    source_port_range          = "*"
  }

  security_rule {
    name                       = "nodes-dns"
    priority                   = 1030
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Udp"
    destination_address_prefix = "*"
    destination_port_range     = "53"
    source_address_prefixes    = local.cluster_ips
    source_port_range          = "*"
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
