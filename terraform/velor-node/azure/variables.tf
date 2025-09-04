variable "region" {
  description = "Azure region"
  type        = string
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 1
}

variable "chain_id" {
  description = "Velor chain ID"
  type        = string
  default     = "TESTING"
}

variable "chain_name" {
  description = "Velor chain name"
  type        = string
  default     = "testnet"
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag for Velor node"
  type        = string
  default     = "devnet"
}

variable "zone_name" {
  description = "Zone name of Azure DNS domain to create records in"
  type        = string
  default     = ""
}

variable "zone_resource_group" {
  description = "Azure resource group name of the DNS zone"
  type        = string
  default     = ""
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  type        = string
  default     = "<workspace>.velor"
}

variable "helm_chart" {
  description = "Path to velor-validator Helm chart file"
  type        = string
  default     = ""
}

variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "helm_values_file" {
  description = "Path to file containing values for Helm chart"
  type        = string
  default     = ""
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "k8s_viewer_groups" {
  description = "List of AD Group IDs to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_debugger_groups" {
  description = "List of AD Group IDs to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "Standard_B8ms"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  type        = number
  default     = 1
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "Standard_F4s_v2"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  type        = string
  default     = 2
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint the instances in the validator nodegroup"
  type        = bool
  default     = false
}
