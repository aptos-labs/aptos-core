output "forge-s3-bucket" {
  value = aws_s3_bucket.aptos-testnet-helm[0].bucket
}
