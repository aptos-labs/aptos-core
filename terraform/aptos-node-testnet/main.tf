terraform {
  backend "s3" {}
}

provider "aws" {
  region = var.region
}

data "aws_caller_identity" "current" {}

locals {
  workspace  = var.workspace_name_override != "" ? var.workspace_name_override : terraform.workspace
  aws_tags   = "Terraform=testnet,Workspace=${local.workspace}"
  chain_name = var.chain_name != "" ? var.chain_name : "${local.workspace}net"

  # Forge assumes the chain_id is 4
  chain_id = var.enable_forge ? 4 : var.chain_id

  # use this map to programatically set helm values
  aptos_node_helm_values_computed = {
    service = {
      validator = {
        external = {
          type = var.enable_forge ? "NodePort" : null
        }
        enableRestApi     = var.enable_forge
        enableMetricsPort = var.enable_forge
      }
      fullnode = {
        external = {
          type = var.enable_forge ? "NodePort" : null
        }
        enableRestApi     = var.enable_forge
        enableMetricsPort = var.enable_forge
      }
    }
  }
}

# merge the operator-provided helm values via TF var
# with the computed helm values
module "aptos-node-helm-values-deepmerge" {
  # https://registry.terraform.io/modules/Invicton-Labs/deepmerge/null/0.1.5
  source = "Invicton-Labs/deepmerge/null"
  maps = [
    local.aptos_node_helm_values_computed,
    var.aptos_node_helm_values,
  ]
}

module "validator" {
  source = "../aptos-node/aws"

  region   = var.region
  iam_path = var.iam_path
  zone_id  = var.zone_id
  # do not create the main fullnode and validator DNS records
  # instead, rely on external-dns from the testnet-addons
  create_records = false
  workspace_dns  = var.workspace_dns

  permissions_boundary_policy = var.permissions_boundary_policy
  workspace_name_override     = var.workspace_name_override

  k8s_api_sources = var.admin_sources_ipv4
  k8s_admin_roles = var.k8s_admin_roles
  k8s_admins      = var.k8s_admins

  chain_id       = local.chain_id
  era            = var.era
  chain_name     = local.chain_name
  image_tag      = var.image_tag
  validator_name = "aptos-node"

  num_validators = var.num_validators
  helm_values    = module.aptos-node-helm-values-deepmerge.merged

  # allow all nodegroups to surge to 2x their size, in case of total nodes replacement
  validator_instance_num = var.num_validator_instance > 0 ? 2 * var.num_validator_instance : var.num_validators
  # create one utility instance per validator, since HAProxy requires resources 1.5 CPU, 2Gi memory for now
  utility_instance_num = var.num_utility_instance > 0 ? var.num_utility_instance : var.num_validators

  utility_instance_type   = var.utility_instance_type
  validator_instance_type = var.validator_instance_type

  # addons
  enable_monitoring      = true
  enable_logger          = true
  monitoring_helm_values = var.monitoring_helm_values
  logger_helm_values     = var.logger_helm_values
}

locals {
  aptos_node_helm_prefix = "${module.validator.helm_release_name}-aptos-node"
}

provider "helm" {
  kubernetes {
    host                   = module.validator.aws_eks_cluster.endpoint
    cluster_ca_certificate = base64decode(module.validator.aws_eks_cluster.certificate_authority.0.data)
    token                  = module.validator.aws_eks_cluster_auth_token
  }
}

provider "kubernetes" {
  host                   = module.validator.aws_eks_cluster.endpoint
  cluster_ca_certificate = base64decode(module.validator.aws_eks_cluster.certificate_authority.0.data)
  token                  = module.validator.aws_eks_cluster_auth_token
}

resource "helm_release" "genesis" {
  name        = "genesis"
  chart       = "${path.module}/../helm/genesis"
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      chain = {
        name     = local.chain_name
        era      = var.era
        chain_id = local.chain_id
      }
      imageTag = var.image_tag
      genesis = {
        numValidators   = var.num_validators
        username_prefix = local.aptos_node_helm_prefix
        domain          = local.domain
        validator = {
          enable_onchain_discovery = false
        }
        fullnode = {
          # only enable onchain discovery if var.zone_id has been provided to provision
          # internet facing network addresses for the fullnodes
          enable_onchain_discovery = var.zone_id != ""
        }
      }
    }),
    jsonencode(var.genesis_helm_values)
  ]
}
