output "forge-s3-bucket" {
  value = aws_s3_bucket.aptos-testnet-helm[0].bucket
}

output "oidc_provider" {
  value     = module.validator.oidc_provider
  sensitive = true
}
