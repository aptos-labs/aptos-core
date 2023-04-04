output "oidc_provider" {
  value     = module.validator.oidc_provider
  sensitive = true
}

output "workspace" {
  value = local.workspace_name
}
