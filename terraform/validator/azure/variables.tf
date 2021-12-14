variable "region" {
  description = "Azure region"
  type        = string
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "zone_name" {
  description = "Zone name of Azure DNS domain to create records in"
  default     = ""
}

variable "zone_resource_group" {
  description = "Azure resource group name of the DNS zone"
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

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "backup_replication_type" {
  default     = "ZRS"
  description = "Replication type of backup storage account"
}

variable "backup_public_access" {
  default     = false
  description = "Allow public access to download backups"
}

variable "k8s_viewer_groups" {
  description = "List of AD Group IDs to configure as Kubernetes viewers"
  type = list(string)
  default = []
}

variable "k8s_debugger_groups" {
  description = "List of AD Group IDs to configure as Kubernetes debuggers"
  type = list(string)
  default = []
}

variable "key_vault_owner_id" {
  default     = null
  description = "Object ID of the key vault owner (defaults to current Azure client)"
}

variable "use_kube_state_metrics" {
  default     = false
  description = "Use kube-state-metrics to monitor k8s cluster"
}
