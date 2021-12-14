resource "random_id" "backup-bucket" {
  byte_length = 4
}

resource "google_storage_bucket" "backup" {
  name                        = "diem-${terraform.workspace}-backup-${random_id.backup-bucket.hex}"
  location                    = var.region
  uniform_bucket_level_access = true
}

resource "google_service_account" "backup" {
  account_id = "diem-${terraform.workspace}-backup"
}

resource "google_storage_bucket_iam_member" "backup" {
  bucket = google_storage_bucket.backup.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.backup.email}"
}

resource "google_service_account_iam_binding" "backup" {
  service_account_id = google_service_account.backup.name
  role               = "roles/iam.workloadIdentityUser"
  members            = ["serviceAccount:${google_container_cluster.diem.workload_identity_config[0].identity_namespace}[default/${terraform.workspace}-diem-validator-backup]"]
}
