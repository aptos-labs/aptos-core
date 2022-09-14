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

variable "do_token" {
  type        = string
  description = "Digital Notion API token"
}

variable "region" {
  description = "Digital Ocean region of nodes"
  type        = string
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
  description = "Machine type for running fullnode"
  default     = "s-16vcpu-32gb"
}
