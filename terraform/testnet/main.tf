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
  aws_tags = "Terraform=testnet,Workspace=${terraform.workspace}"
}

module "validator" {
  source = "../validator/aws"

  region                      = var.region
  iam_path                    = var.iam_path
  permissions_boundary_policy = var.permissions_boundary_policy

  validator_name        = "testnet"
  helm_enable_validator = false
  helm_release_name     = "val0"

  ssh_sources_ipv4   = var.admin_sources_ipv4
  vault_sources_ipv4 = var.admin_sources_ipv4
  k8s_api_sources    = var.admin_sources_ipv4
  k8s_admin_roles    = var.k8s_admin_roles
  k8s_admins         = var.k8s_admins
  ssh_pub_key        = var.ssh_pub_key

  max_node_pool_surge = var.enable_forge ? 2 : 1
  node_pool_sizes = var.validator_lite_mode ? {
    utilities  = var.num_utilities_instance > 0 ? var.num_utilities_instance : 3
    validators = var.num_validator_instance > 0 ? var.num_validator_instance : var.num_validators + var.num_public_fullnodes
    } : {
    utilities  = var.num_utilities_instance > 0 ? var.num_utilities_instance : 3 * var.num_validators
    validators = var.num_validator_instance > 0 ? var.num_validator_instance : 3 * var.num_validators + var.num_public_fullnodes + 1
  }
  vault_lb_internal       = false
  utility_instance_type   = var.utility_instance_type
  validator_instance_type = var.validator_instance_type
  trusted_instance_type   = var.trusted_instance_type
}

data "aws_eks_cluster" "aptos" {
  name = "aptos-${terraform.workspace}"
}

data "aws_eks_cluster_auth" "aptos" {
  name = data.aws_eks_cluster.aptos.name
}

provider "helm" {
  kubernetes {
    host                   = module.validator.kubernetes.kubernetes_host
    cluster_ca_certificate = module.validator.kubernetes.kubernetes_ca_cert
    token                  = data.aws_eks_cluster_auth.aptos.token
  }
}

provider "kubernetes" {
  host                   = module.validator.kubernetes.kubernetes_host
  cluster_ca_certificate = module.validator.kubernetes.kubernetes_ca_cert
  token                  = data.aws_eks_cluster_auth.aptos.token
}

resource "helm_release" "testnet" {
  name        = "aptos"
  chart       = "${path.module}/testnet"
  max_history = var.enable_forge ? 2 : 10
  wait        = false

  values = [
    jsonencode({
      imageTag          = local.image_tag
      validatorLite     = var.validator_lite_mode
      chain_name        = "aptos-${terraform.workspace}"
      localVaultBackend = var.enable_dev_vault # Toggle dev vault mode
      genesis = {
        numValidators      = var.num_validators
        numPublicFullnodes = var.num_public_fullnodes
        era                = var.era
        chain_id           = var.chain_id
        vaultRoleId        = vault_approle_auth_backend_role.genesis-reset-role.role_id
        vaultSecretId      = vault_approle_auth_backend_role_secret_id.genesis-reset-id.secret_id
      }
      vault = {
        server           = module.validator.vault.server
        tls              = module.validator.vault.tls
        prometheusTarget = module.validator.vault.prometheusTarget
      }
      service = {
        domain   = local.domain
        aws_tags = local.aws_tags
        fullnode = {
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
        vpc_id       = module.validator.vpc_id
        role_arn     = aws_iam_role.k8s-aws-integrations.arn
        zone_name    = var.zone_id != "" ? data.aws_route53_zone.aptos[0].name : null
      }
    }),
    jsonencode(var.testnet_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

resource "helm_release" "validator" {
  count       = var.num_validators
  name        = "val${count.index}"
  chart       = var.validator_lite_mode ? "${path.module}/../helm/validator-lite" : "${path.module}/../helm/validator"
  max_history = var.enable_forge ? 2 : 10
  wait        = false

  values = [
    module.validator.helm_values,
    jsonencode({
      exposeValidatorRestApi = var.enable_forge
      localVaultBackend      = var.enable_dev_vault # Toggle dev vault mode
      validator = {
        name = "val${count.index}"
      }
      chain = {
        name     = "aptos-${terraform.workspace}"
        era      = var.era
        chain_id = var.chain_id
      }
      imageTag = local.image_tag
      service = {
        external = {
          type = "NodePort"
        }
      }
      monitoring = {
        fullKubernetesScrape = false
        haproxy = {
          clientCertVerificationDisabled = true
        }
      }
      vault = {
        serverIPRanges = []
        namespace      = "val${count.index}"
        auth = {
          mount_path = "auth/kubernetes-val${count.index}"
          config = {
            role = "val${count.index}-<role>"
          }
        }
        nodeSelector = jsondecode(module.validator.helm_values)["validator"]["nodeSelector"]
        tolerations  = jsondecode(module.validator.helm_values)["validator"]["tolerations"]
        resources = var.enable_dev_vault ? {
          limits = {
            cpu    = "0.5"
            memory = "2Gi"
          }
          requests = {
            cpu    = "0.5"
            memory = "2Gi"
          }
        } : {}
      }
      backup = {
        enable = count.index == 0
      }
    }),
    jsonencode(var.validator_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

resource "helm_release" "public-fullnode" {
  count       = var.num_public_fullnodes
  name        = "pfn${count.index}"
  chart       = "${path.module}/../helm/fullnode"
  max_history = var.enable_forge ? 2 : 10
  wait        = false

  values = [
    jsonencode(var.public_fullnode_helm_values),
    jsonencode(jsondecode(module.validator.helm_values)["fullnode"]),
    jsonencode({
      chain = {
        name             = "aptos-${terraform.workspace}"
        era              = var.era
        genesisConfigmap = "aptos-testnet-genesis-e${var.era}"
      }
      aptos_chains = {
        "aptos-${terraform.workspace}" = {
          seeds = []
        }
      }
      image = {
        tag = local.image_tag
      }
      logging = {
        address = var.enable_pfn_logger ? "testnet-pfn-aptos-logger:5044" : ""
      }
    }),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}


resource "helm_release" "pfn-logger" {
  count       = var.enable_pfn_logger ? 1 : 0
  name        = "testnet-pfn"
  chart       = "${path.module}/../../helm/logger"
  max_history = var.enable_forge ? 2 : 10
  wait        = false

  values = [
    jsonencode(var.pfn_logger_helm_values),
    jsonencode({
      logger = {
        name = "novi-testnet-pfn"
      }
      chain = {
        name = "aptos-${terraform.workspace}"
      }
    }),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = "${path.module}/forge"
  max_history = 2
  wait        = false

  depends_on = [
    null_resource.helm-s3-package
  ]

  values = [
    jsonencode({
      forge = {
        numValidators = var.num_validators
        helmBucket    = aws_s3_bucket.aptos-testnet-helm[0].bucket
        image = {
          tag = local.image_tag
        }
      }
      serviceAccount = {
        annotations = {
          "eks.amazonaws.com/role-arn" = aws_iam_role.forge[0].arn
        }
      }
      vault = {
        server = module.validator.vault.server
        tls    = module.validator.vault.tls
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}
