variable "ssh_keys" {
  description = "Map of username to SSH public keys to configure for SSH access"
  default     = {}
}

resource "tls_private_key" "ca-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_self_signed_cert" "ca" {
  key_algorithm         = "ECDSA"
  private_key_pem       = tls_private_key.ca-key.private_key_pem
  validity_period_hours = 10 * 365 * 24
  early_renewal_hours   = 1 * 365 * 24
  is_ca_certificate     = true
  allowed_uses          = ["cert_signing"]

  subject {
    common_name  = "Vault CA"
    organization = "diem-${terraform.workspace}"
  }
}

resource "local_file" "ca" {
  filename        = "${terraform.workspace}-vault.ca"
  content         = tls_self_signed_cert.ca.cert_pem
  file_permission = "0644"
}

resource "tls_private_key" "vault-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_cert_request" "vault" {
  key_algorithm   = tls_private_key.vault-key.algorithm
  private_key_pem = tls_private_key.vault-key.private_key_pem
  dns_names       = ["localhost"]
  ip_addresses    = [google_compute_address.vault-lb.address, "127.0.0.1"]

  subject {
    common_name  = "vault"
    organization = "diem-${terraform.workspace}"
  }
}

resource "tls_locally_signed_cert" "vault" {
  cert_request_pem      = tls_cert_request.vault.cert_request_pem
  ca_key_algorithm      = tls_private_key.ca-key.algorithm
  ca_private_key_pem    = tls_private_key.ca-key.private_key_pem
  ca_cert_pem           = tls_self_signed_cert.ca.cert_pem
  validity_period_hours = tls_self_signed_cert.ca.validity_period_hours
  early_renewal_hours   = tls_self_signed_cert.ca.early_renewal_hours
  allowed_uses          = ["server_auth"]
}

resource "google_secret_manager_secret" "vault-tls" {
  provider  = google-beta
  secret_id = "diem-${terraform.workspace}-vault-tls"
  replication {
    automatic = true
  }
}

resource "google_secret_manager_secret_version" "vault-tls" {
  provider    = google-beta
  secret      = google_secret_manager_secret.vault-tls.id
  secret_data = tls_private_key.vault-key.private_key_pem
}

resource "google_secret_manager_secret_iam_member" "vault" {
  provider  = google-beta
  secret_id = google_secret_manager_secret.vault-tls.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.vault.email}"
}

resource "random_id" "key" {
  byte_length = 4
}

resource "google_kms_key_ring" "diem" {
  name     = "diem-${terraform.workspace}"
  location = var.keyring_location
}

resource "google_kms_crypto_key" "vault" {
  name     = "diem-${terraform.workspace}-vault-${random_id.key.hex}"
  key_ring = google_kms_key_ring.diem.self_link

  lifecycle {
    prevent_destroy = true
  }
}

resource "google_spanner_instance" "diem" {
  config       = var.spanner_config
  display_name = "diem-${terraform.workspace}"
}

resource "google_spanner_database" "vault" {
  instance = google_spanner_instance.diem.name
  name     = "vault"
  ddl = [
    "CREATE TABLE Vault (Key STRING(MAX) NOT NULL, Value BYTES(MAX)) PRIMARY KEY (Key)",
    "CREATE TABLE VaultHA (Key STRING(MAX) NOT NULL, Value STRING(MAX), Identity STRING(36) NOT NULL, Timestamp TIMESTAMP NOT NULL) PRIMARY KEY (Key)",
  ]

  lifecycle {
    prevent_destroy = true
  }
}

resource "google_spanner_database_iam_member" "vault" {
  instance = google_spanner_instance.diem.name
  database = google_spanner_database.vault.name
  role     = "roles/spanner.databaseUser"
  member   = "serviceAccount:${google_service_account.vault.email}"
}

resource "google_service_account" "vault" {
  account_id = "diem-${terraform.workspace}-vault"
}

resource "google_kms_crypto_key_iam_member" "vault" {
  crypto_key_id = google_kms_crypto_key.vault.id
  role          = "roles/cloudkms.cryptoKeyEncrypterDecrypter"
  member        = "serviceAccount:${google_service_account.vault.email}"
}

data "google_compute_lb_ip_ranges" "ranges" {}

resource "google_compute_firewall" "bastion-ssh" {
  name    = "diem-${terraform.workspace}-bastion-ssh"
  network = google_compute_network.diem.id

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  source_ranges = var.ssh_sources_ipv4
  target_tags   = ["bastion"]
}

resource "google_compute_firewall" "bastion-vault-ssh" {
  name    = "diem-${terraform.workspace}-bastion-vault-ssh"
  network = google_compute_network.diem.id

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  source_tags = ["bastion"]
  target_tags = ["vault"]
}

resource "google_compute_firewall" "vault-api" {
  name    = "diem-${terraform.workspace}-vault-api"
  network = google_compute_network.diem.id

  allow {
    protocol = "tcp"
    ports    = ["8200"]
  }

  source_ranges = concat([google_container_cluster.diem.cluster_ipv4_cidr], data.google_compute_lb_ip_ranges.ranges.http_ssl_tcp_internal)
  target_tags   = ["vault"]
}

resource "google_compute_firewall" "vault-ha" {
  name    = "diem-${terraform.workspace}-vault-ha"
  network = google_compute_network.diem.id

  allow {
    protocol = "tcp"
    ports    = ["8200", "8201"]
  }

  source_tags = ["vault"]
  target_tags = ["vault"]
}

data "google_compute_image" "debian" {
  project = "debian-cloud"
  family  = "debian-10"
}

resource "google_compute_instance" "bastion" {
  count        = var.bastion_enable ? 1 : 0
  name         = "diem-${terraform.workspace}-bastion"
  zone         = local.zone
  machine_type = "f1-micro"
  tags         = ["bastion"]

  boot_disk {
    initialize_params {
      image = data.google_compute_image.debian.self_link
      type  = "pd-standard"
    }
  }

  network_interface {
    network = google_compute_network.diem.id
    access_config {}
  }

  metadata = {
    ssh-keys = join("\n", [for user, sshkey in var.ssh_keys : "${user}:${sshkey}"])
  }

  metadata_startup_script = file("${path.module}/templates/bastion_user_data.sh")
}

data "template_file" "vault_user_data" {
  template = file("${path.module}/templates/vault_user_data.sh")

  vars = {
    vault_version    = "1.8.1"
    vault_sha256     = "bb411f2bbad79c2e4f0640f1d3d5ef50e2bda7d4f40875a56917c95ff783c2db"
    vault_ca         = tls_self_signed_cert.ca.cert_pem
    vault_cert       = tls_locally_signed_cert.vault.cert_pem
    vault_key_secret = google_secret_manager_secret.vault-tls.secret_id
    vault_config = jsonencode({
      cluster_addr = "https://$LOCAL_IPV4:8201"
      api_addr     = "https://${google_compute_address.vault-lb.address}:8200"
      storage = {
        spanner = {
          ha_enabled = "true"
          # google_spanner_database.vault.id is supposed to be this whole string, but it's not
          database = "projects/${var.project}/instances/${google_spanner_instance.diem.name}/databases/${google_spanner_database.vault.name}"
        }
      }
      listener = {
        tcp = {
          address       = "[::]:8200"
          tls_cert_file = "/etc/vault/vault.crt"
          tls_key_file  = "/etc/vault/vault.key"
          telemetry = {
            unauthenticated_metrics_access = true
          }
        }
      }
      seal = {
        gcpckms = {
          project    = var.project
          region     = var.keyring_location
          key_ring   = google_kms_key_ring.diem.name
          crypto_key = google_kms_crypto_key.vault.name
        }
      }
      telemetry = {
        disable_hostname = true
      }
    })
  }
}

resource "google_compute_instance_template" "vault" {
  name_prefix  = "diem-${terraform.workspace}-vault-"
  tags         = ["vault"]
  machine_type = "n1-standard-1"

  disk {
    source_image = data.google_compute_image.debian.self_link
    disk_type    = "pd-standard"
    boot         = true
  }

  network_interface {
    network = google_compute_network.diem.name
  }

  service_account {
    email  = google_service_account.vault.email
    scopes = ["cloud-platform"]
  }

  metadata = {
    ssh-keys = join("\n", [for user, sshkey in var.ssh_keys : "${user}:${sshkey}"])
  }

  metadata_startup_script = data.template_file.vault_user_data.rendered

  lifecycle {
    create_before_destroy = true
  }
}

resource "google_compute_health_check" "vault" {
  name = "diem-${terraform.workspace}-vault"

  https_health_check {
    port         = "8200"
    request_path = "/v1/sys/health?standbyok=true&uninitcode=200"
  }
}

resource "google_compute_instance_group_manager" "vault" {
  name               = "diem-${terraform.workspace}-vault"
  base_instance_name = "diem-${terraform.workspace}-vault"
  zone               = local.zone
  target_size        = var.vault_num

  version {
    instance_template = google_compute_instance_template.vault.self_link
  }

  auto_healing_policies {
    health_check      = google_compute_health_check.vault.self_link
    initial_delay_sec = 300
  }
}

resource "google_compute_health_check" "vault-active" {
  name = "diem-${terraform.workspace}-vault-active"

  https_health_check {
    port         = "8200"
    request_path = "/v1/sys/health"
  }
}

resource "google_compute_region_backend_service" "vault" {
  name          = "diem-${terraform.workspace}-vault"
  health_checks = [google_compute_health_check.vault-active.self_link]

  backend {
    group = google_compute_instance_group_manager.vault.instance_group
  }
}

resource "google_compute_address" "vault-lb" {
  name         = "diem-${terraform.workspace}-vault-lb"
  address_type = "INTERNAL"
  subnetwork   = data.google_compute_subnetwork.region.self_link
}

resource "google_compute_forwarding_rule" "vault" {
  name                  = "diem-${terraform.workspace}-vault"
  network               = google_compute_network.diem.name
  backend_service       = google_compute_region_backend_service.vault.self_link
  load_balancing_scheme = "INTERNAL"
  ip_address            = google_compute_address.vault-lb.address
  ports                 = ["8200"]
}
