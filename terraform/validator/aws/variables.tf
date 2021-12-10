variable "region" {
  description = "AWS region"
  type        = string
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "zone_id" {
  description = "Zone ID of Route 53 domain to create records in"
  default     = ""
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  default     = "<workspace>.diem"
}

variable "helm_chart" {
  description = "Path to diem-validator Helm chart file"
  default     = "../helm"
}

variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "helm_values_file" {
  description = "Path to file containing values for Helm chart"
  default     = ""
}

variable "helm_force_update" {
  description = "Force Terraform to update the Helm deployment"
  default     = false
}

variable "k8s_admins" {
  description = "List of AWS usernames to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_admin_roles" {
  description = "List of AWS roles to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_viewers" {
  description = "List of AWS usernames to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_viewer_roles" {
  description = "List of AWS roles to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_debuggers" {
  description = "List of AWS usernames to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "k8s_debugger_roles" {
  description = "List of AWS roles to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "iam_path" {
  default     = "/"
  description = "Path to use when naming IAM objects"
}

variable "permissions_boundary_policy" {
  default     = ""
  description = "ARN of IAM policy to set as permissions boundary on created roles"
}

variable "vpc_cidr_block" {
  default     = "192.168.0.0/16"
  description = "VPC CIDR Block"
}

variable "helm_enable_validator" {
  description = "Enable deployment of the validator Helm chart"
  default     = true
}

variable "helm_release_name" {
  description = "Override the Helm release name used when referencing Kubernetes service accounts"
  default     = ""
}

variable "vault_lb_internal" {
  description = "[TESTNET USE ONLY] Whether the Vault load balancer should be internal-only or external"
  default     = true
}

variable "vault_sources_ipv4" {
  description = "[TESTNET USE ONLY] List of external CIDR subnets which can access the Vault API"
  default     = []
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.medium"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c5.xlarge"
}

variable "trusted_instance_type" {
  description = "Instance type used for trusted components"
  default     = "c5.large"
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "max_node_pool_surge" {
  default     = 1
  description = "Multiplier on the max size of the node pool"
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}
