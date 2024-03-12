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

variable "era" {
  description = "Chain era, used to start a clean chain"
  type        = number
  default     = 1
}

variable "chain_id" {
  description = "Aptos chain ID"
  type        = string
  default     = "TESTING"
}

variable "chain_name" {
  description = "Aptos chain name"
  type        = string
  default     = "testnet"
}

variable "validator_name" {
  description = "Name of the validator node owner"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag for Aptos node"
  type        = string
  default     = "devnet"
}

variable "helm_chart" {
  description = "Path to aptos-validator Helm chart file"
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

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  type        = string
  default     = "n2-standard-8"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  type        = number
  default     = 1
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint the instances in the utility nodegroup"
  type        = bool
  default     = false
}

variable "utility_instance_disk_size_gb" {
  description = "Disk size for utility instances"
  type        = number
  default     = 20
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "n2-standard-32"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  type        = number
  default     = 2
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  type        = bool
  default     = false
}

variable "validator_instance_disk_size_gb" {
  description = "Disk size for validator instances"
  type        = number
  default     = 20
}

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  type        = bool
  default     = true
}

### DNS

variable "zone_name" {
  description = "Zone name of GCP Cloud DNS zone to create records in"
  type        = string
  default     = ""
}

variable "zone_project" {
  description = "GCP project which the DNS zone is in (if different)"
  type        = string
  default     = ""
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  type        = bool
  default     = true
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  type        = string
  default     = "<workspace>.aptos"
}

variable "create_dns_records" {
  description = "Creates DNS records in var.zone_name that point to k8s service, as opposed to using external-dns or other means"
  type        = bool
  default     = true
}

variable "dns_ttl" {
  description = "Time-to-Live for the Validator and Fullnode DNS records"
  type        = number
  default     = 300
}

### Autoscaling

variable "gke_enable_node_autoprovisioning" {
  description = "Enable node autoprovisioning for GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/how-to/node-auto-provisioning"
  type        = bool
  default     = false
}

variable "gke_node_autoprovisioning_max_cpu" {
  description = "Maximum CPU utilization for GKE node_autoprovisioning"
  type        = number
  default     = 10
}

variable "gke_node_autoprovisioning_max_memory" {
  description = "Maximum memory utilization for GKE node_autoprovisioning"
  type        = number
  default     = 100
}

variable "gke_enable_autoscaling" {
  description = "Enable autoscaling for the nodepools in the GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-autoscaler"
  type        = bool
  default     = true
}

variable "gke_autoscaling_profile" {
  description = "Autoscaling profile for GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-autoscaler#autoscaling_profiles"
  type        = string
  default     = "BALANCED"
}

variable "gke_autoscaling_max_node_count" {
  description = "Maximum number of nodes for GKE nodepool autoscaling"
  type        = number
  default     = 10
}

### Naming overrides

variable "helm_release_name_override" {
  description = "If set, overrides the name of the aptos-node helm chart"
  type        = string
  default     = ""
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

### GKE cluster config

variable "cluster_ipv4_cidr_block" {
  description = "The IP address range of the container pods in this cluster, in CIDR notation. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#cluster_ipv4_cidr_block"
  type        = string
  default     = ""
}

variable "enable_calico" {
  description = "Enable Calico for network policy enforcement"
  type        = bool
  default     = false
}

### Helm

variable "num_validators" {
  description = "The number of validator nodes to create"
  type        = number
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  type        = number
  default     = 1
}

variable "gke_maintenance_policy" {
  description = "The maintenance policy to use for the cluster. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#maintenance_policy"
  type = object({
    recurring_window = object({
      start_time = string
      end_time   = string
      recurrence = string
    })
  })
  default = {
    recurring_window = {
      start_time = "2023-06-15T00:00:00Z"
      end_time   = "2023-06-15T23:59:00Z"
      recurrence = "FREQ=DAILY"
    }
  }
}
