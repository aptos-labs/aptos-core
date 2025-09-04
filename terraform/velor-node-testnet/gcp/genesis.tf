resource "google_storage_bucket" "genesis" {
  count                       = var.enable_forge ? 1 : 0
  name                        = "velor-${terraform.workspace}-genesis"
  project                     = var.project
  location                    = var.region
  uniform_bucket_level_access = true

  lifecycle_rule {
    condition {
      age = 7
    }
    action {
      type = "Delete"
    }
  }
}

resource "google_storage_bucket_iam_binding" "genesis" {
  count  = var.enable_forge ? 1 : 0
  bucket = google_storage_bucket.genesis[0].name
  role   = "roles/storage.objectAdmin"
  members = [
    "serviceAccount:${google_service_account.signed-url[0].email}",
  ]
}


resource "google_storage_bucket" "signed-url" {
  count                       = var.enable_forge ? 1 : 0
  name                        = "velor-${terraform.workspace}-signed-url-gcf"
  project                     = var.project
  location                    = var.region
  uniform_bucket_level_access = true
}

data "archive_file" "signed-url" {
  count       = var.enable_forge ? 1 : 0
  type        = "zip"
  source_dir  = "${path.module}/functions/signed-url"
  output_path = "/tmp/function-source.zip"
}

resource "google_storage_bucket_object" "signed-url" {
  count        = var.enable_forge ? 1 : 0
  name         = "function-source.zip"
  bucket       = google_storage_bucket.signed-url[0].name
  source       = data.archive_file.signed-url[0].output_path
  content_type = "application/zip"
}

resource "google_service_account" "signed-url" {
  count      = var.enable_forge ? 1 : 0
  project    = var.project
  account_id = "velor-${local.workspace_name}-signed-url"
}

resource "google_project_iam_member" "signed-url" {
  count   = var.enable_forge ? 1 : 0
  project = var.project
  role    = "roles/iam.serviceAccountTokenCreator"
  member  = "serviceAccount:${google_service_account.signed-url[0].email}"
}

resource "google_cloudfunctions2_function" "signed-url" {
  count    = var.enable_forge ? 1 : 0
  project  = var.project
  name     = "velor-${local.workspace_name}-signed-url"
  location = var.region

  build_config {
    runtime     = "python312"
    entry_point = "handler"
    source {
      storage_source {
        bucket = google_storage_bucket.signed-url[0].name
        object = google_storage_bucket_object.signed-url[0].name
      }
    }
  }
  service_config {
    ingress_settings      = "ALLOW_INTERNAL_ONLY"
    service_account_email = google_service_account.signed-url[0].email
    environment_variables = {
      BUCKET_NAME = google_storage_bucket.genesis[0].name
    }
  }
}

resource "google_cloud_run_service_iam_member" "member" {
  count    = var.enable_forge ? 1 : 0
  project  = var.project
  location = google_cloudfunctions2_function.signed-url[0].location
  service  = google_cloudfunctions2_function.signed-url[0].name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
