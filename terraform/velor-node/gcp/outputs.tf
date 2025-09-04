output "helm_release_name" {
  value = helm_release.validator.name
}

output "gke_cluster_name" {
  value = google_container_cluster.velor.name
}

output "gke_cluster_endpoint" {
  value = google_container_cluster.velor.endpoint
}

output "gke_cluster_ca_certificate" {
  value = google_container_cluster.velor.master_auth[0].cluster_ca_certificate
}

output "gke_cluster_workload_identity_config" {
  value = google_container_cluster.velor.workload_identity_config
}
