output "oidc_provider" {
  value = local.oidc_provider
}

output "kubernetes_host" {
  value = module.eks.kubernetes.kubernetes_host
}

output "kubernetes_ca_certificate" {
  value = module.eks.kubernetes.kubernetes_ca_cert
}

output "kubernetes_token" {
  value = data.aws_eks_cluster_auth.velor.token
}

output "s3_backup_role" {
  value = aws_iam_role.backup
}

output "s3_backup_bucket" {
  value = aws_s3_bucket.backup
}
