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

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  default     = true
}

variable "dns_prefix_name" {
  description = "DNS prefix for fullnode url"
  default     = "fullnode"
}

variable "zone_name" {
  description = "Zone name of GCP Cloud DNS zone to create records in"
  default     = ""
}

variable "zone_project" {
  description = "GCP project which the DNS zone is in (if different)"
  default     = ""
}

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

variable "num_extra_instance" {
  default     = 0
  description = "Number of extra instances to add into node pool"
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

variable "chain_name" {
  description = "Aptos chain name"
  default     = "devnet"
}

variable "machine_type" {
  description = "Machine type for running fullnode"
  default     = "c2-standard-16"
}

variable "enable_backup" {
  description = "enable data backup from fullnode"
  default     = false
}
