output "vpc_id" {
  value     = module.validator.vpc_id
  sensitive = true
}

output "aws_subnet_private" {
  value = module.validator.aws_subnet_private
}

output "cluster_security_group_id" {
  value = module.validator.cluster_security_group_id
}

output "oidc_provider" {
    value = module.validator.oidc_provider
}
