resource "google_container_cluster" "diem" {
  provider = google-beta
  name     = "diem-${terraform.workspace}"
  location = local.zone
  network  = google_compute_network.diem.id

  remove_default_node_pool = true
  initial_node_count       = 1
  logging_service          = "none"
  monitoring_service       = "none"

  release_channel {
    channel = "REGULAR"
  }

  master_auth {
    username = ""
    password = ""
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
    identity_namespace = "${var.project}.svc.id.goog"
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

  lifecycle {
    prevent_destroy = true
  }
}

resource "google_container_node_pool" "utilities" {
  provider   = google-beta
  name       = "utilities"
  location   = local.zone
  cluster    = google_container_cluster.diem.name
  node_count = lookup(var.node_pool_sizes, "utilities", 3)

  node_config {
    machine_type    = "e2-custom-2-4096"
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 20
    service_account = google_service_account.gke.email
    tags            = ["utilities"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      node_metadata = "GKE_METADATA_SERVER"
    }
  }
}

resource "google_container_node_pool" "validators" {
  provider   = google-beta
  name       = "validators"
  location   = local.zone
  cluster    = google_container_cluster.diem.name
  node_count = lookup(var.node_pool_sizes, "validators", 3)

  node_config {
    machine_type    = "c2-standard-4"
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 20
    service_account = google_service_account.gke.email
    tags            = ["validators"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      node_metadata = "GKE_METADATA_SERVER"
    }

    taint {
      key    = "diem.org/nodepool"
      value  = "validators"
      effect = "NO_EXECUTE"
    }
  }
}

resource "google_container_node_pool" "trusted" {
  provider   = google-beta
  name       = "trusted"
  location   = local.zone
  cluster    = google_container_cluster.diem.name
  node_count = lookup(var.node_pool_sizes, "trusted", 1)

  node_config {
    machine_type    = "n2-custom-2-4096"
    image_type      = "COS_CONTAINERD"
    disk_size_gb    = 20
    service_account = google_service_account.gke.email
    tags            = ["trusted"]

    shielded_instance_config {
      enable_secure_boot = true
    }

    workload_metadata_config {
      node_metadata = "GKE_METADATA_SERVER"
    }

    taint {
      key    = "diem.org/nodepool"
      value  = "trusted"
      effect = "NO_EXECUTE"
    }
  }
}
