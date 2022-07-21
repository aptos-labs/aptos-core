resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = "${path.module}/../helm/forge"
  max_history = 2
  wait        = false

  values = [
    jsonencode({
      forge = {
        image = {
          tag = var.image_tag
        }
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

