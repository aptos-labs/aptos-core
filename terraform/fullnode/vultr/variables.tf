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
  default     = "aptos"
  description = "Kubernetes namespace that the fullnode will be deployed into"
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "num_fullnodes" {
  default     = 1
  description = "Number of fullnodes"
}

variable "image_tag" {
  default     = "devnet"
  description = "Docker image tag to use for the fullnode"
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 1
}

variable "chain_id" {
  description = "aptos chain ID"
  default     = "DEVNET"
}

variable "machine_type" {
  description = "Machine type for running fullnode. All configurations can be obtained at https://www.vultr.com/api/#tag/plans"
  default     = "vc2-16c-32gb"
}

variable "api_key" {
  description = "API Key, can be obtained at https://my.vultr.com/settings/#settingsapi"
  default     = ""
}

variable "fullnode_region" {
  description = "Geographical region for the node location. All 25 regions can be obtained at https://api.vultr.com/v2/regions"
  default     = "fra"
}


variable "block_storage_class" {
  description = "Either vultr-block-storage for high_perf/ssd, vultr-block-storage-hdd for storage_opt/hdd. high_perf is not available in all regions!"
  default     = "vultr-block-storage"
}
