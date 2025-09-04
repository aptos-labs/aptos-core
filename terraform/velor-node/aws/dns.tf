resource "time_sleep" "lb_creation" {
  create_duration = "2m"
  depends_on      = [helm_release.validator]
}

resource "random_string" "validator-dns" {
  upper   = false
  special = false
  length  = 16
}

data "aws_route53_zone" "velor" {
  count   = var.zone_id != "" ? 1 : 0
  zone_id = var.zone_id
}

locals {
  dns_prefix  = var.workspace_dns ? "${local.workspace_name}." : ""
  record_name = replace(var.record_name, "<workspace>", local.workspace_name)
  # domain name for external-dns, if it is installed
  domain = var.zone_id != "" ? "${local.dns_prefix}${data.aws_route53_zone.velor[0].name}" : null
}

data "kubernetes_service" "validator-lb" {
  count = var.zone_id == "" || !var.create_records ? 0 : 1
  metadata {
    # This is the main validator LB service that is created by the velor-node helm chart
    name = "${local.workspace_name}-velor-node-0-validator-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

data "kubernetes_service" "fullnode-lb" {
  count = var.zone_id == "" || !var.create_records ? 0 : 1
  metadata {
    # This is the main fullnode LB service that is created by the velor-node helm chart
    name = "${local.workspace_name}-velor-node-0-fullnode-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

resource "aws_route53_record" "validator" {
  count   = var.zone_id == "" || !var.create_records ? 0 : 1
  zone_id = var.zone_id
  name    = "${random_string.validator-dns.result}.${local.record_name}"
  type    = "CNAME"
  ttl     = 3600
  records = [data.kubernetes_service.validator-lb[0].status[0].load_balancer[0].ingress[0].hostname]
}

resource "aws_route53_record" "fullnode" {
  count   = var.zone_id == "" || !var.create_records ? 0 : 1
  zone_id = var.zone_id
  name    = local.record_name
  type    = "CNAME"
  ttl     = 3600
  records = [data.kubernetes_service.fullnode-lb[0].status[0].load_balancer[0].ingress[0].hostname]
}
