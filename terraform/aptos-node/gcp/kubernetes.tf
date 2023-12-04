provider "kubernetes" {
  host                   = "https://${google_container_cluster.aptos.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
  token                  = data.google_client_config.provider.access_token
}

resource "kubernetes_storage_class" "ssd" {
  metadata {
    name = "ssd"
  }
  storage_provisioner = "kubernetes.io/gce-pd"
  volume_binding_mode = "WaitForFirstConsumer"
  parameters = {
    type = "pd-ssd"
  }
}

provider "helm" {
  kubernetes {
    host                   = "https://${google_container_cluster.aptos.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.aptos.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

locals {
  # helm chart paths
  aptos_node_helm_chart_path = var.helm_chart != "" ? var.helm_chart : "${path.module}/../../helm/aptos-node"

  # override the helm release name if an override exists, otherwise adopt the workspace name
  helm_release_name = var.helm_release_name_override != "" ? var.helm_release_name_override : local.workspace_name
}

resource "helm_release" "validator" {
  name        = local.helm_release_name
  chart       = local.aptos_node_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      numValidators     = var.num_validators
      numFullnodeGroups = var.num_fullnode_groups
      imageTag          = var.image_tag
      manageImages      = var.manage_via_tf # if we're managing the entire deployment via terraform, override the images as well
      chain = {
        era      = var.era
        chain_id = var.chain_id
        name     = var.chain_name
      }
      validator = {
        name = var.validator_name
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
        }
        nodeSelector = var.validator_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "validators"
        } : {}
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      fullnode = {
        storage = {
          class = kubernetes_storage_class.ssd.metadata[0].name
        }
        nodeSelector = var.validator_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "validators"
        } : {}
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = "validators"
          effect = "NoExecute"
        }]
      }
      haproxy = {
        nodeSelector = var.utility_instance_enable_taint ? {
          "cloud.google.com/gke-nodepool" = "utilities"
        } : {}
        tolerations = [{
          key    = "aptos.org/nodepool"
          value  = "utilities"
          effect = "NoExecute"
        }]
      }
      service = {
        domain = local.domain
      }
    }),
    var.helm_values_file != "" ? file(var.helm_values_file) : "{}",
    jsonencode(var.helm_values),
  ]

  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.aptos_node_helm_chart_path, "**") : filesha1("${local.aptos_node_helm_chart_path}/${f}")]))
    }
  }
}
