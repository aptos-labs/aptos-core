variable "kubernetes_host" {
  type        = string
  description = "URL of Kubernetes master API"
}

variable "kubernetes_ca_cert" {
  type        = string
  description = "PEM certificate for the Kubernetes CA"
}

variable "issuer" {
  default     = ""
  description = "JWT issuer"
}

variable "service_account_prefix" {
  type        = string
  description = "Prefix for Diem service accounts, e.g. default-diem-validator"
}

variable "pod_cidrs" {
  default     = []
  description = "List of IP CIDRs which are allowed to authenticate"
}

resource "vault_auth_backend" "kubernetes" {
  type = "kubernetes"
  path = "kubernetes-${var.namespace}"
}

resource "vault_kubernetes_auth_backend_config" "kubernetes" {
  backend            = vault_auth_backend.kubernetes.path
  kubernetes_host    = var.kubernetes_host
  kubernetes_ca_cert = var.kubernetes_ca_cert
  issuer             = var.issuer
}

resource "vault_kubernetes_auth_backend_role" "safety-rules" {
  backend                          = vault_auth_backend.kubernetes.path
  role_name                        = "${var.namespace}-safety-rules"
  bound_service_account_names      = ["${var.service_account_prefix}-safety-rules"]
  bound_service_account_namespaces = ["*"]
  token_bound_cidrs                = var.pod_cidrs
  token_period                     = 3600
  token_policies                   = [vault_policy.safety-rules.name]
}

resource "vault_kubernetes_auth_backend_role" "validator" {
  backend                          = vault_auth_backend.kubernetes.path
  role_name                        = "${var.namespace}-validator"
  bound_service_account_names      = ["${var.service_account_prefix}-validator"]
  bound_service_account_namespaces = ["*"]
  token_bound_cidrs                = var.pod_cidrs
  token_period                     = 3600
  token_policies                   = [vault_policy.validator.name]
}

resource "vault_kubernetes_auth_backend_role" "fullnode" {
  backend                          = vault_auth_backend.kubernetes.path
  role_name                        = "${var.namespace}-fullnode"
  bound_service_account_names      = ["${var.service_account_prefix}-fullnode"]
  bound_service_account_namespaces = ["*"]
  token_bound_cidrs                = var.pod_cidrs
  token_period                     = 3600
  token_policies                   = [vault_policy.fullnode.name]
}

resource "vault_kubernetes_auth_backend_role" "key-manager" {
  backend                          = vault_auth_backend.kubernetes.path
  role_name                        = "${var.namespace}-key-manager"
  bound_service_account_names      = ["${var.service_account_prefix}-key-manager"]
  bound_service_account_namespaces = ["*"]
  token_bound_cidrs                = var.pod_cidrs
  token_period                     = 3600
  token_policies                   = [vault_policy.key-manager.name]
}

variable "depends_on_" {
  description = "Dummy variable used by testnet Terraform"
  type        = list(string)
  default     = []
}

output "kubernetes_auth_path" {
  value = vault_auth_backend.kubernetes.path
}
