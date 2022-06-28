output "forge-s3-bucket" {
  value = length(aws_s3_bucket.aptos-testnet-helm) > 0 ? aws_s3_bucket.aptos-testnet-helm[0].bucket : null
}

output "oidc_provider" {
  value     = module.validator.oidc_provider
  sensitive = true
}

output "workspace" {
  value = local.workspace
}
