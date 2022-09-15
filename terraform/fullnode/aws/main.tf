terraform {
  backend "s3" {}
}

provider "aws" {
  region = var.region
}

data "aws_caller_identity" "current" {}

data "aws_ecr_image" "stable" {
  count           = var.ecr_repo != "" ? 1 : 0
  repository_name = var.ecr_repo
  image_tag       = "stable"
}

locals {
  image_tag = var.image_tag != "" ? var.image_tag : (var.ecr_repo != ""
    ? [for t in data.aws_ecr_image.stable[0].image_tags : t if substr(t, 0, 5) == "main_"][0]
    : "latest"
  )
  aws_tags       = "Terraform=pfn,Workspace=${local.workspace_name}"
  workspace_name = var.workspace_name_override == "" ? terraform.workspace : var.workspace_name_override
}

module "eks" {
  source                      = "../../modules/eks"
  region                      = var.region
  workspace_name_override     = "pfn-${local.workspace_name}"
  eks_cluster_name            = "aptos-pfn-${local.workspace_name}"
  iam_path                    = var.iam_path
  k8s_admins                  = var.k8s_admins
  k8s_admin_roles             = var.k8s_admin_roles
  permissions_boundary_policy = var.permissions_boundary_policy
  utility_instance_type       = var.utility_instance_type
  fullnode_instance_type      = var.fullnode_instance_type
  num_fullnodes               = var.num_fullnodes
  num_extra_instance          = var.num_extra_instance
}

data "aws_eks_cluster" "aptos" {
  depends_on = [
    module.eks
  ]
  name = "aptos-pfn-${local.workspace_name}"
}

data "aws_eks_cluster_auth" "aptos" {
  name = data.aws_eks_cluster.aptos.name
}

provider "helm" {
  kubernetes {
    host                   = module.eks.kubernetes.kubernetes_host
    cluster_ca_certificate = module.eks.kubernetes.kubernetes_ca_cert
    token                  = data.aws_eks_cluster_auth.aptos.token
  }
}

provider "kubernetes" {
  host                   = module.eks.kubernetes.kubernetes_host
  cluster_ca_certificate = module.eks.kubernetes.kubernetes_ca_cert
  token                  = data.aws_eks_cluster_auth.aptos.token
}

