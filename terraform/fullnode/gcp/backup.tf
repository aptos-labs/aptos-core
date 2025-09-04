resource "random_id" "backup-bucket" {
  byte_length = 4
}

resource "google_storage_bucket" "backup" {
  name                        = "velor-${terraform.workspace}-backup-${random_id.backup-bucket.hex}"
  location                    = var.region
  uniform_bucket_level_access = true
}

resource "google_service_account" "backup" {
  account_id = "velor-${terraform.workspace}-backup"
}

resource "google_storage_bucket_iam_member" "backup" {
  bucket = google_storage_bucket.backup.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.backup.email}"
}

resource "google_service_account_iam_binding" "backup" {
  service_account_id = google_service_account.backup.name
  role               = "roles/iam.workloadIdentityUser"
  members            = [for i in range(var.num_fullnodes) : "serviceAccount:${google_container_cluster.velor.workload_identity_config[0].workload_pool}[${var.k8s_namespace}/pfn${i}-velor-fullnode]"]
}

# backup public access
resource "google_storage_bucket_iam_member" "public" {
  bucket = google_storage_bucket.backup.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}
