module "explorer" {
  count = var.enable_explorer ? 1 : 0

  # TF module by git ref clones the module into .terraform/
  # It will try to use the locally cloned one despite changes in remote. To update it:
  #   terraform get -update
  source = "git@github.com:diem/explorer.git//terraform/modules/explorer?ref=main"

  image_repo = var.explorer_image_repo
  image_tag  = var.explorer_image_tag
  hostname   = "explorer.${local.domain}"

  # service
  service_type = "NodePort"

  # use the ALB
  create_ingress = true
  ingress_type = "alb"
  ingress_annotations = {
    "kubernetes.io/ingress.class"               = "alb"
    "alb.ingress.kubernetes.io/scheme"          = "internet-facing"
    "alb.ingress.kubernetes.io/tags"            = local.aws_tags
    "alb.ingress.kubernetes.io/inbound-cidrs"   = join(",", var.client_sources_ipv4)
    "external-dns.alpha.kubernetes.io/hostname" = "explorer.${local.domain}"
    "alb.ingress.kubernetes.io/certificate-arn" = var.zone_id != "" ? aws_acm_certificate.ingress[0].arn : null
  }

  chain                  = "DPN"
  env                    = "Dev"
  base_url               = "https://explorer.${local.domain}"
  graphql_url            = ""
  blockchain_jsonrpc_url = "https://${local.domain}"
  blockchain_restapi_url = "https://api.${local.domain}"

  create_service_account      = false
  service_account_name        = "aptos-testnet"
  service_account_annotations = {}
}
