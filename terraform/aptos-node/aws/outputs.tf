output "helm_release_name" {
  value = helm_release.validator[0].name
}

output "aws_eks_cluster" {
  value     = aws_eks_cluster.aptos
  sensitive = true
}

output "aws_eks_cluster_auth_token" {
  value     = data.aws_eks_cluster_auth.aptos.token
  sensitive = true
}

output "oidc_provider" {
  value     = local.oidc_provider
  sensitive = true
}


### Node outputs

output "validator_endpoint" {
  value = var.zone_id == "" || !var.create_records ? null : "/dns4/${aws_route53_record.validator[0].fqdn}/tcp/${data.kubernetes_service.validator-lb[0].spec[0].port[0].port}"
}

output "fullnode_endpoint" {
  value = var.zone_id == "" || !var.create_records ? null : "/dns4/${aws_route53_record.fullnode[0].fqdn}/tcp/${data.kubernetes_service.fullnode-lb[0].spec[0].port[0].port}"
}

### Network outputs

output "vpc_id" {
  value     = aws_vpc.vpc.id
  sensitive = true
}

output "aws_subnet_public" {
  value = aws_subnet.public
}

output "aws_subnet_private" {
  value = aws_subnet.private
}

output "aws_vpc_cidr_block" {
  value = aws_vpc.vpc.cidr_block
}

output "aws_eip_nat_public_ip" {
  value = aws_eip.nat.public_ip
}

output "cluster_security_group_id" {
  value = aws_eks_cluster.aptos.vpc_config[0].cluster_security_group_id
}
