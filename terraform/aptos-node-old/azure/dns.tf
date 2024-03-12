resource "time_sleep" "lb_creation" {
  create_duration = "1m"
  depends_on      = [helm_release.validator]
}

resource "random_string" "validator-dns" {
  upper   = false
  special = false
  length  = 16
}

locals {
  record_name = replace(var.record_name, "<workspace>", terraform.workspace)
}

data "kubernetes_service" "validator-lb" {
  count = var.zone_name != "" ? 1 : 0
  metadata {
    name = "${terraform.workspace}-aptos-node-validator-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

data "kubernetes_service" "fullnode-lb" {
  count = var.zone_name != "" ? 1 : 0
  metadata {
    name = "${terraform.workspace}-aptos-node-fullnode-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

resource "azurerm_dns_a_record" "validator" {
  count               = var.zone_name != "" ? 1 : 0
  resource_group_name = var.zone_resource_group
  zone_name           = var.zone_name
  name                = "${random_string.validator-dns.result}.${local.record_name}"
  ttl                 = 3600
  records             = [data.kubernetes_service.validator-lb[0].status[0].load_balancer[0].ingress[0].ip]
}

resource "azurerm_dns_a_record" "fullnode" {
  count               = var.zone_name != "" ? 1 : 0
  resource_group_name = var.zone_resource_group
  zone_name           = var.zone_name
  name                = local.record_name
  ttl                 = 3600
  records             = [data.kubernetes_service.fullnode-lb[0].status[0].load_balancer[0].ingress[0].ip]
}

output "validator_endpoint" {
  value = var.zone_name != "" ? "/dns4/${trimsuffix(azurerm_dns_a_record.validator[0].fqdn, ".")}/tcp/${data.kubernetes_service.validator-lb[0].spec[0].port[0].port}" : null
}

output "fullnode_endpoint" {
  value = var.zone_name != "" ? "/dns4/${trimsuffix(azurerm_dns_a_record.fullnode[0].fqdn, ".")}/tcp/${data.kubernetes_service.fullnode-lb[0].spec[0].port[0].port}" : null
}
