data "aws_route53_zone" "pfn" {
  count   = var.zone_id != "" ? 1 : 0
  zone_id = var.zone_id
}

locals {
  dns_prefix = var.workspace_dns ? "${local.workspace_name}.${var.dns_prefix_name}." : "${var.dns_prefix_name}."
  domain     = var.zone_id != "" ? "${local.dns_prefix}${data.aws_route53_zone.pfn[0].name}" : null
}

resource "aws_acm_certificate" "ingress" {
  count = var.zone_id != "" ? 1 : 0

  domain_name               = local.domain
  subject_alternative_names = concat(["*.${local.domain}"], var.tls_sans)
  validation_method         = "DNS"

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Terraform = "pfn"
    Workspace = terraform.workspace
  }
}

resource "aws_route53_record" "ingress-acm-validation" {
  for_each = var.zone_id == "" ? {} : { for dvo in aws_acm_certificate.ingress[0].domain_validation_options : dvo.domain_name => dvo }

  zone_id         = var.zone_id
  allow_overwrite = true
  name            = each.value.resource_record_name
  type            = each.value.resource_record_type
  records         = [each.value.resource_record_value]
  ttl             = 60
}

resource "aws_acm_certificate_validation" "ingress" {
  count = var.zone_id != "" ? 1 : 0

  certificate_arn         = aws_acm_certificate.ingress[0].arn
  validation_record_fqdns = [for dvo in aws_acm_certificate.ingress[0].domain_validation_options : dvo.resource_record_name]
  depends_on              = [aws_route53_record.ingress-acm-validation]
}
