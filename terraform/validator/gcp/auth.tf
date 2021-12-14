resource "google_service_account" "gke" {
  account_id = "diem-${terraform.workspace}-gke"
}

resource "google_project_iam_member" "gke-logging" {
  role   = "roles/logging.logWriter"
  member = "serviceAccount:${google_service_account.gke.email}"
}

resource "google_project_iam_member" "gke-metrics" {
  role   = "roles/monitoring.metricWriter"
  member = "serviceAccount:${google_service_account.gke.email}"
}

resource "google_project_iam_member" "gke-monitoring" {
  role   = "roles/monitoring.viewer"
  member = "serviceAccount:${google_service_account.gke.email}"
}

resource "google_project_iam_custom_role" "k8s-debugger" {
  role_id     = "container.debugger"
  title       = "Kubernetes Engine Debugger"
  description = "Additional permissions to debug Kubernetes Engine workloads"
  permissions = [
    "container.pods.exec",
    "container.pods.portForward",
  ]
}
