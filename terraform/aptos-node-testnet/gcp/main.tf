data "google_client_config" "provider" {} # new token

provider "kubernetes" {
  host                   = "https://${module.validator.gke_cluster_endpoint}"
  cluster_ca_certificate = base64decode(module.validator.gke_cluster_ca_certificate)
  token                  = data.google_client_config.provider.access_token
}

provider "helm" {
  kubernetes {
    host                   = "https://${module.validator.gke_cluster_endpoint}"
    cluster_ca_certificate = base64decode(module.validator.gke_cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

module "validator" {
  source = "../../aptos-node/gcp"

  cluster_bootstrap = var.cluster_bootstrap
  manage_via_tf     = var.manage_via_tf

  # Project config
  project = var.project
  zone    = var.zone
  region  = var.region

  # DNS
  zone_name     = var.zone_name # keep empty if you don't want a DNS name
  zone_project  = var.zone_project
  record_name   = var.record_name
  workspace_dns = var.workspace_dns
  # dns_prefix_name = var.dns_prefix_name
  # do not create the main fullnode and validator DNS records
  # instead, rely on external-dns from the testnet-addons
  create_dns_records = var.create_dns_records
  dns_ttl            = var.dns_ttl

  # General chain config
  era            = var.era
  chain_id       = var.chain_id
  chain_name     = var.chain_name
  image_tag      = var.image_tag
  validator_name = "aptos-node"

  # K8s config
  k8s_api_sources         = var.k8s_api_sources
  cluster_ipv4_cidr_block = var.cluster_ipv4_cidr_block

  # autoscaling
  gke_enable_node_autoprovisioning     = var.gke_enable_node_autoprovisioning
  gke_node_autoprovisioning_max_cpu    = var.gke_node_autoprovisioning_max_cpu
  gke_node_autoprovisioning_max_memory = var.gke_node_autoprovisioning_max_memory
  gke_enable_autoscaling               = var.gke_enable_autoscaling
  gke_autoscaling_max_node_count       = var.gke_autoscaling_max_node_count

  # Testnet config
  workspace_name_override = var.workspace_name_override
  # if forge enabled, standardize the helm release name for ease of operations
  helm_release_name_override = var.enable_forge ? "aptos-node" : ""
  helm_values                = var.aptos_node_helm_values
  num_validators             = var.num_validators
  num_fullnode_groups        = var.num_fullnode_groups

  # Instance config
  utility_instance_type   = var.utility_instance_type
  validator_instance_type = var.validator_instance_type

  # addons
  enable_monitoring      = var.enable_monitoring
  enable_node_exporter   = var.enable_prometheus_node_exporter
  monitoring_helm_values = var.monitoring_helm_values

  gke_maintenance_policy = var.gke_maintenance_policy
}

locals {
  genesis_helm_chart_path = "${path.module}/../../helm/genesis"

  workspace_name = var.workspace_name_override != "" ? var.workspace_name_override : terraform.workspace
  chain_name     = var.chain_name != "" ? var.chain_name : "${local.workspace_name}net"

  # Forge assumes the chain_id is 4
  chain_id = var.enable_forge ? 4 : var.chain_id

  aptos_node_helm_prefix = var.enable_forge ? "aptos-node" : "${module.validator.helm_release_name}-aptos-node"
}


resource "helm_release" "genesis" {
  name        = "genesis"
  chart       = local.genesis_helm_chart_path
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
          # only enable onchain discovery if var.zone_name has been provided to provision
          # internet facing network addresses for the fullnodes
          enable_onchain_discovery = var.zone_name != ""
        }
      }
    }),
    jsonencode(var.genesis_helm_values)
  ]

  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.genesis_helm_chart_path, "**") : filesha1("${local.genesis_helm_chart_path}/${f}")]))
    }
  }
}
