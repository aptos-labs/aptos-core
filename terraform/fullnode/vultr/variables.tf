variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "fullnode_helm_values" {
  description = "Map of values to pass to public fullnode Helm"
  type        = any
  default     = {}
}

variable "fullnode_helm_values_list" {
  description = "List of values to pass to public fullnode, for setting different value per node. length(fullnode_helm_values_list) must equal var.num_fullnodes"
  type        = any
  default     = {}
}

variable "k8s_namespace" {
  description = "Kubernetes namespace that the fullnode will be deployed into"
  type        = string
  default     = "velor"
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "num_fullnodes" {
  description = "Number of fullnodes"
  type        = number
  default     = 1
}

variable "image_tag" {
  description = "Docker image tag to use for the fullnode"
  type        = string
  default     = "devnet"
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 1
}

variable "chain_id" {
  description = "Velor chain ID"
  type        = string
  default     = "DEVNET"
}

variable "chain_name" {
  description = "Velor chain name"
  type        = string
  default     = "devnet"
}

variable "machine_type" {
  description = "Machine type for running fullnode. All configurations can be obtained at https://www.vultr.com/api/#tag/plans"
  type        = string
  default     = "vc2-16c-32gb"
}

variable "api_key" {
  description = "API Key, can be obtained at https://my.vultr.com/settings/#settingsapi"
  type        = string
  default     = ""
}

variable "fullnode_region" {
  description = "Geographical region for the node location. All 25 regions can be obtained at https://api.vultr.com/v2/regions"
  type        = string
  default     = "fra"
}


variable "block_storage_class" {
  description = "Either vultr-block-storage for high_perf/ssd, vultr-block-storage-hdd for storage_opt/hdd. high_perf is not available in all regions!"
  type        = string
  default     = "vultr-block-storage"
}
