resource "google_container_cluster" "aptos" {
  provider = google-beta
  name     = "aptos-${terraform.workspace}"
  location = local.zone
  network  = google_compute_network.aptos.id

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
    enable_private_nodes    = true
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

  pod_security_policy_config {
    enabled = true
  }
}

resource "google_container_node_pool" "fullnodes" {
  provider   = google-beta
  name       = "fullnodes"
  location   = local.zone
  cluster    = google_container_cluster.aptos.name
  node_count = var.num_fullnodes + var.num_extra_instance

  node_config {
    machine_type    = var.machine_type
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 100
    service_account = google_service_account.gke.email
    tags            = ["fullnodes"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      mode = "GKE_METADATA"
    }
  }
}
