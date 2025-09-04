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
  dns_prefix  = var.workspace_dns ? "${local.workspace_name}." : ""
  record_name = replace(var.record_name, "<workspace>", local.workspace_name)
  domain      = var.zone_name != "" ? "${local.dns_prefix}${data.google_dns_managed_zone.velor[0].dns_name}" : null
}

data "kubernetes_service" "validator-lb" {
  count = var.zone_name != "" && var.create_dns_records ? 1 : 0
  metadata {
    name = "${local.workspace_name}-velor-node-0-validator-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

data "kubernetes_service" "fullnode-lb" {
  count = var.zone_name != "" && var.create_dns_records ? 1 : 0
  metadata {
    name = "${local.workspace_name}-velor-node-0-fullnode-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

data "google_dns_managed_zone" "velor" {
  count   = var.zone_name != "" ? 1 : 0
  name    = var.zone_name
  project = var.zone_project != "" ? var.zone_project : var.project
}

resource "google_dns_record_set" "validator" {
  count        = var.zone_name != "" && var.create_dns_records ? 1 : 0
  managed_zone = data.google_dns_managed_zone.velor[0].name
  project      = data.google_dns_managed_zone.velor[0].project
  name         = "${random_string.validator-dns.result}.${local.record_name}.${data.google_dns_managed_zone.velor[0].dns_name}"
  type         = "A"
  ttl          = var.dns_ttl
  rrdatas      = [data.kubernetes_service.validator-lb[0].status[0].load_balancer[0].ingress[0].ip]
}

resource "google_dns_record_set" "fullnode" {
  count        = var.zone_name != "" && var.create_dns_records ? 1 : 0
  managed_zone = data.google_dns_managed_zone.velor[0].name
  project      = data.google_dns_managed_zone.velor[0].project
  name         = "${local.record_name}.${data.google_dns_managed_zone.velor[0].dns_name}"
  type         = "A"
  ttl          = var.dns_ttl
  rrdatas      = [data.kubernetes_service.fullnode-lb[0].status[0].load_balancer[0].ingress[0].ip]
}

output "validator_endpoint" {
  value = var.zone_name != "" && var.create_dns_records ? "/dns4/${trimsuffix(google_dns_record_set.validator[0].name, ".")}/tcp/${data.kubernetes_service.validator-lb[0].spec[0].port[0].port}" : null
}

output "fullnode_endpoint" {
  value = var.zone_name != "" && var.create_dns_records ? "/dns4/${trimsuffix(google_dns_record_set.fullnode[0].name, ".")}/tcp/${data.kubernetes_service.fullnode-lb[0].spec[0].port[0].port}" : null
}
