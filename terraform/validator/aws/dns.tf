resource "time_sleep" "lb_creation" {
  create_duration = "2m"
  depends_on      = [helm_release.validator]
}

resource "random_string" "validator-dns" {
  upper   = false
  special = false
  length  = 16
}

locals {
  record_name = replace(var.record_name, "<workspace>", local.workspace_name)
}

data "kubernetes_service" "validator-lb" {
  count = var.zone_id != "" ? 1 : 0
  metadata {
    name = "${local.workspace_name}-diem-validator-validator-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

data "kubernetes_service" "fullnode-lb" {
  count = var.zone_id != "" ? 1 : 0
  metadata {
    name = "${local.workspace_name}-diem-validator-fullnode-lb"
  }
  depends_on = [time_sleep.lb_creation]
}

resource "aws_route53_record" "validator" {
  count   = var.zone_id != "" ? 1 : 0
  zone_id = var.zone_id
  name    = "${random_string.validator-dns.result}.${local.record_name}"
  type    = "CNAME"
  ttl     = 3600
  records = [data.kubernetes_service.validator-lb[0].status[0].load_balancer[0].ingress[0].hostname]
}

resource "aws_route53_record" "fullnode" {
  count   = var.zone_id != "" ? 1 : 0
  zone_id = var.zone_id
  name    = local.record_name
  type    = "CNAME"
  ttl     = 3600
  records = [data.kubernetes_service.fullnode-lb[0].status[0].load_balancer[0].ingress[0].hostname]
}

output "validator_endpoint" {
  value = var.zone_id != "" ? "/dns4/${aws_route53_record.validator[0].fqdn}/tcp/${data.kubernetes_service.validator-lb[0].spec[0].port[0].port}" : null
}

output "fullnode_endpoint" {
  value = var.zone_id != "" ? "/dns4/${aws_route53_record.fullnode[0].fqdn}/tcp/${data.kubernetes_service.fullnode-lb[0].spec[0].port[0].port}" : null
}
