locals {
  forge_helm_chart_path = "${path.module}/../helm/forge"
}
resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = local.forge_helm_chart_path
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

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.forge_helm_chart_path, "**") : filesha1("${local.forge_helm_chart_path}/${f}")]))
  }
}

