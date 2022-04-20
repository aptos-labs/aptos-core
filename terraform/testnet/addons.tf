resource "helm_release" "metrics-server" {
  count       = var.enable_k8s_metrics_server ? 1 : 0
  name        = "metrics-server"
  namespace   = "kube-system"
  chart       = "${path.module}/../helm/k8s-metrics"
  max_history = 10
  wait        = false

  values = [
    jsonencode({
      coredns = {
        maxReplicas = var.num_validators
        minReplicas = var.coredns_min_replicas
      }
    })
  ]
}
