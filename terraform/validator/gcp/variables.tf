variable "project" {
  description = "GCP project"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
}

variable "zone" {
  description = "GCP zone suffix"
  type        = string
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "zone_name" {
  description = "Zone name of GCP Cloud DNS zone to create records in"
  default     = ""
}

variable "zone_project" {
  description = "GCP project which the DNS zone is in (if different)"
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

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "ssh_sources_ipv4" {
  description = "List of CIDR subnets which can SSH to the bastion host"
  default     = ["0.0.0.0/0"]
}

variable "bastion_enable" {
  default     = false
  description = "Enable the bastion host for access to Vault"
}

variable "vault_num" {
  default     = 1
  description = "Number of Vault servers"
}

variable "spanner_config" {
  default = "regional-us-central1"
}

variable "keyring_location" {
  default = "us-central1"
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}
