### Project config

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

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  default     = true
}

### Chain config

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

variable "image_tag" {
  description = "Docker image tag for Aptos node"
  default     = "devnet"
}

### DNS config

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

### Testnet config

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "helm_release_name_override" {
  description = "If set, overrides the name of the aptos-node helm chart"
  default     = ""
}

variable "aptos_node_helm_values" {
  description = "Map of values to pass to aptos-node helm chart"
  type        = any
  default     = {}
}

variable "genesis_helm_values" {
  description = "Map of values to pass to genesis helm chart"
  type        = any
  default     = {}
}

variable "forge_helm_values" {
  description = "Map of values to pass to Forge Helm"
  type        = any
  default     = {}
}

variable "num_validators" {
  description = "The number of validator nodes to create"
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  default     = 1
}


### K8s config

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

### Instance config

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "n2-standard-8"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "n2-standard-32"
}

### Addons

variable "enable_forge" {
  description = "Enable Forge"
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring Helm"
  type        = any
  default     = {}
}

### Autoscaling

variable "gke_enable_node_autoprovisioning" {
  description = "Enable node autoprovisioning for GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/how-to/node-auto-provisioning"
  default     = false
}

variable "gke_node_autoprovisioning_max_cpu" {
  description = "Maximum CPU utilization for GKE node_autoprovisioning"
  default     = 10
}

variable "gke_node_autoprovisioning_max_memory" {
  description = "Maximum memory utilization for GKE node_autoprovisioning"
  default     = 100
}

variable "gke_enable_autoscaling" {
  description = "Enable autoscaling for the nodepools in the GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-autoscaler"
  default     = true
}

variable "gke_autoscaling_max_node_count" {
  description = "Maximum number of nodes for GKE nodepool autoscaling"
  default     = 10
}

### GKE cluster config

variable "cluster_ipv4_cidr_block" {
  description = "The IP address range of the container pods in this cluster, in CIDR notation. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#cluster_ipv4_cidr_block"
  default     = ""
}
