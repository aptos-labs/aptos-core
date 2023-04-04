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
  value = data.aws_eks_cluster_auth.aptos.token
}
