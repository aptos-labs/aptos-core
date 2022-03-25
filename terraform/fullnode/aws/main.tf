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
  aws_tags = "Terraform=pfn,Workspace=${terraform.workspace}"
}

module "eks" {
  source                      = "../../modules/eks"
  region                      = var.region
  workspace_name_override     = "pfn-${terraform.workspace}"
  eks_cluster_name            = "aptos-pfn-${terraform.workspace}"
  iam_path                    = var.iam_path
  k8s_admins                  = var.k8s_admins
  k8s_admin_roles             = var.k8s_admin_roles
  permissions_boundary_policy = var.permissions_boundary_policy
  utility_instance_type       = var.utility_instance_type
  fullnode_instance_type      = var.fullnode_instance_type
  num_fullnodes               = var.num_fullnodes
}

data "aws_eks_cluster" "aptos" {
  name = "aptos-pfn-${terraform.workspace}"
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

resource "helm_release" "pfn" {
  name        = "aptos"
  chart       = "${path.module}/fullnode"
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      imageTag          = local.image_tag
      service = {
        domain   = local.domain
        aws_tags = local.aws_tags
        fullnode = {
          numFullnodes = var.num_fullnodes
          loadBalancerSourceRanges = var.client_sources_ipv4
        }
        monitoring = {
          loadBalancerSourceRanges = var.admin_sources_ipv4
        }
      }
      ingress = {
        acm_certificate          = var.zone_id != "" ? aws_acm_certificate.ingress[0].arn : null
        loadBalancerSourceRanges = var.client_sources_ipv4
      }
      monitoring = {
        prometheus = {
          storage = {
            class = "gp2"
          }
        }
      }
      aws = {
        region       = var.region
        cluster_name = data.aws_eks_cluster.aptos.name
        vpc_id       = module.eks.vpc_id
        role_arn     = aws_iam_role.k8s-aws-integrations.arn
        zone_name    = var.zone_id != "" ? data.aws_route53_zone.pfn[0].name : null
      }
    }),
    jsonencode(var.pfn_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

resource "helm_release" "fullnode" {
  count       = var.num_fullnodes
  name        = "pfn${count.index}"
  chart       = "${path.module}/../../helm/fullnode"
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      chain = {
        era  = var.era
      }
      image = {
        tag = local.image_tag
      }
      logging = {
        address = var.enable_pfn_logger ? "fullnode-pfn-aptos-logger:5044" : ""
      }
      nodeSelector = {
        "eks.amazonaws.com/nodegroup" = "fullnode"
      }
      storage = {
        class = "gp2"
      }
    }),
    jsonencode(var.fullnode_helm_values),
    jsonencode(var.fullnode_helm_values_list == {} ? {} : var.fullnode_helm_values_list[count.index]),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}


resource "helm_release" "pfn-logger" {
  count       = var.enable_pfn_logger ? 1 : 0
  name        = "pfn-logger"
  chart       = "${path.module}/../../helm/logger"
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      logger = {
        name = "pfn"
      }
      chain = {
        name = "aptos-${terraform.workspace}"
      }
    }),
    jsonencode(var.pfn_logger_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}
