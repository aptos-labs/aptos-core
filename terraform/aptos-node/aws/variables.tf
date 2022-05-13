variable "region" {
  description = "AWS region"
  type        = string
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default = 1
}

variable "chain_id" {
  description = "Aptos chain ID"
  default = "TESTING"
}

variable "chain_name" {
  description = "Aptos chain name"
  default = "testnet"
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag for Aptos node"
  default     = "devnet"
}

variable "zone_id" {
  description = "Zone ID of Route 53 domain to create records in"
  default     = ""
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  default     = "<workspace>.aptos"
}

variable "helm_chart" {
  description = "Path to aptos-validator Helm chart file"
  default     = ""
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
  default     = true
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

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.medium"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  default     = 1
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c5.xlarge"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  default     = 2
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "enable_logger" {
  description = "Enable logger helm chart"
  default     = false
}

variable "logger_helm_values" {
  description = "Map of values to pass to logger Helm"
  type        = any
  default     = {}
}

variable "enable_monitoring" {
  description = "Enable monitoring helm chart"
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring Helm"
  type        = any
  default     = {}
}
