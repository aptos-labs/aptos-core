resource "google_container_cluster" "aptos" {
  provider = google-beta
  name     = "aptos-${terraform.workspace}"
  location = local.zone
  network  = google_compute_network.aptos.id

  lifecycle {
    ignore_changes = [
      private_cluster_config,
      cluster_autoscaling[0].auto_provisioning_defaults[0].shielded_instance_config
    ]
    prevent_destroy = true
  }

  remove_default_node_pool = true
  initial_node_count       = 1
  logging_service          = "logging.googleapis.com/kubernetes"
  monitoring_service       = "monitoring.googleapis.com/kubernetes"

  release_channel {
    channel = "REGULAR"
  }

  master_auth {
    client_certificate_config {
      issue_client_certificate = false
    }
  }

  master_authorized_networks_config {
    dynamic "cidr_blocks" {
      for_each = var.k8s_api_sources
      content {
        cidr_block = cidr_blocks.value
      }
    }
  }

  private_cluster_config {
    enable_private_nodes    = var.gke_enable_private_nodes
    enable_private_endpoint = false
    master_ipv4_cidr_block  = "172.16.0.0/28"
  }

  ip_allocation_policy {
    cluster_ipv4_cidr_block = ""
  }

  workload_identity_config {
    workload_pool = "${var.project}.svc.id.goog"
  }

  addons_config {
    network_policy_config {
      disabled = false
    }
  }

  network_policy {
    enabled  = true
    provider = "CALICO"
  }

  cluster_autoscaling {
    enabled = var.gke_enable_node_autoprovisioning

    dynamic "resource_limits" {
      for_each = var.gke_enable_node_autoprovisioning ? {
        "cpu"    = var.gke_node_autoprovisioning_max_cpu
        "memory" = var.gke_node_autoprovisioning_max_memory
      } : {}
      content {
        resource_type = resource_limits.key
        minimum       = 1
        maximum       = resource_limits.value
      }
    }
    auto_provisioning_defaults {
      oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]
      service_account = google_service_account.gke.email
    }
  }
}

resource "google_container_node_pool" "fullnodes" {
  provider   = google-beta
  name       = "fullnodes"
  location   = local.zone
  cluster    = google_container_cluster.aptos.name
  node_count = var.gke_enable_autoscaling ? null : var.num_fullnodes + var.num_extra_instance

  node_config {
    machine_type    = var.machine_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = var.instance_disk_size_gb
    service_account = google_service_account.gke.email
    tags            = ["fullnodes"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      mode = "GKE_METADATA"
    }
  }

  dynamic "autoscaling" {
    for_each = var.gke_enable_autoscaling ? [1] : []
    content {
      min_node_count = 1
      max_node_count = var.gke_autoscaling_max_node_count
    }
  }
}
