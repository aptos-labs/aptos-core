provider "kubernetes" {
  host                   = "https://${google_container_cluster.velor.endpoint}"
  cluster_ca_certificate = base64decode(google_container_cluster.velor.master_auth[0].cluster_ca_certificate)
  token                  = data.google_client_config.provider.access_token
}

resource "kubernetes_namespace" "velor" {
  metadata {
    name = var.k8s_namespace
  }
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
    host                   = "https://${google_container_cluster.velor.endpoint}"
    cluster_ca_certificate = base64decode(google_container_cluster.velor.master_auth[0].cluster_ca_certificate)
    token                  = data.google_client_config.provider.access_token
  }
}

locals {
  fullnode_helm_chart_path   = "${path.module}/../../helm/fullnode"
  pfn_addons_helm_chart_path = "${path.module}/../../helm/pfn-addons"
  monitoring_helm_chart_path = "${path.module}/../../helm/monitoring"

  utility_nodeSelector = var.utility_instance_enable_taint ? {
    "cloud.google.com/gke-nodepool" = "utilities"
  } : {}
  utility_tolerations = [{
    key    = "velor.org/nodepool"
    value  = "utilities"
    effect = "NoExecute"
  }]
}

resource "helm_release" "fullnode" {
  count            = var.num_fullnodes
  name             = "pfn${count.index}"
  chart            = local.fullnode_helm_chart_path
  max_history      = 10
  wait             = false
  namespace        = var.k8s_namespace
  create_namespace = true

  values = [
    jsonencode({
      imageTag     = var.image_tag
      manageImages = var.manage_via_tf # if we're managing the entire deployment via terraform, override the images as well
      chain = {
        era  = var.era
        name = var.chain_name
      }
      image = {
        tag = var.image_tag
      }
      nodeSelector = var.fullnode_instance_enable_taint ? {
        "cloud.google.com/gke-nodepool" = "fullnodes"
      } : {}
      tolerations = [{
        key    = "velor.org/nodepool"
        value  = "fullnodes"
        effect = "NoExecute"
      }]
      storage = {
        class = kubernetes_storage_class.ssd.metadata[0].name
      }
      service = {
        type = "LoadBalancer"
        annotations = {
          "external-dns.alpha.kubernetes.io/hostname" = var.zone_name != "" ? "pfn${count.index}.${local.domain}" : ""
        }
      }
      backup = {
        # only enable backup for fullnode 0
        enable       = count.index == var.backup_fullnode_index ? var.enable_backup : false
        nodeSelector = local.utility_nodeSelector
        tolerations  = local.utility_tolerations
        config = {
          location = "gcs"
          gcs = {
            bucket = google_storage_bucket.backup.name
          }
        }
      }
      backup_verify = {
        nodeSelector = local.utility_nodeSelector
        tolerations  = local.utility_tolerations
      }
      backup_compaction = {
        nodeSelector = local.utility_nodeSelector
        tolerations  = local.utility_tolerations
      }
      restore = {
        config = {
          location = "gcs"
          gcs = {
            bucket = google_storage_bucket.backup.name
          }
        }
      }
      serviceAccount = {
        annotations = {
          "iam.gke.io/gcp-service-account" = google_service_account.backup.email
        }
      }
    }),
    jsonencode(var.fullnode_helm_values),
    jsonencode(var.fullnode_helm_values_list == {} ? {} : var.fullnode_helm_values_list[count.index]),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  dynamic "set" {
    for_each = var.manage_via_tf ? toset([""]) : toset([])
    content {
      # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
      name  = "chart_sha1"
      value = sha1(join("", [for f in fileset(local.fullnode_helm_chart_path, "**") : filesha1("${local.fullnode_helm_chart_path}/${f}")]))
    }
  }
}

resource "helm_release" "monitoring" {
  count       = var.enable_monitoring ? 1 : 0
  name        = "velor-monitoring"
  chart       = local.monitoring_helm_chart_path
  max_history = 5
  wait        = false
  namespace   = var.k8s_namespace

  values = [
    jsonencode({
      chain = {
        name = var.chain_name
      }
      fullnode = {
        name = var.fullnode_name
      }
      service = {
        domain = local.domain
      }
      monitoring = {
        prometheus = {
          storage = {
            class = "standard"
          }
        }
      }
      kube-state-metrics = {
        enabled = var.enable_kube_state_metrics
      }
      prometheus-node-exporter = {
        enabled = var.enable_prometheus_node_exporter
      }
    }),
    jsonencode(var.monitoring_helm_values),
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.monitoring_helm_chart_path, "**") : filesha1("${local.monitoring_helm_chart_path}/${f}")]))
  }
}
