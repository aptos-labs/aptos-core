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

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 1
}

variable "chain_id" {
  description = "Aptos chain ID"
  default     = "TESTING"
}

variable "chain_name" {
  description = "Aptos chain name"
  default     = "testnet"
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag for Aptos node"
  default     = "devnet"
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

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "n2-standard-8"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  default     = 1
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint the instances in the utility nodegroup"
  default     = false
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c2-standard-16"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  default     = 2
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  default     = false
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

variable "enable_node_exporter" {
  description = "Enable Prometheus node exporter helm chart"
  default     = false
}

variable "node_exporter_helm_values" {
  description = "Map of values to pass to node exporter Helm"
  type        = any
  default     = {}
}
