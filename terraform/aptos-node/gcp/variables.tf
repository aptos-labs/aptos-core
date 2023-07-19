### Project config

variable "cluster_bootstrap" {
  description = "Set when bootstrapping a new cluster"
  type        = bool
  default     = false
}

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

variable "utility_instance_disk_size_gb" {
  description = "Disk size for utility instances"
  default     = 20
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "n2-standard-32"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  default     = 2
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  default     = false
}

variable "validator_instance_disk_size_gb" {
  description = "Disk size for validator instances"
  default     = 20
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

variable "manage_via_tf" {
  description = "Whether to manage the aptos-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  default     = true
}

### DNS

variable "zone_name" {
  description = "Zone name of GCP Cloud DNS zone to create records in"
  default     = ""
}

variable "zone_project" {
  description = "GCP project which the DNS zone is in (if different)"
  default     = ""
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  default     = true
}

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  default     = "<workspace>.aptos"
}

variable "create_dns_records" {
  description = "Creates DNS records in var.zone_name that point to k8s service, as opposed to using external-dns or other means"
  default     = true
}

variable "dns_ttl" {
  description = "Time-to-Live for the Validator and Fullnode DNS records"
  default     = 300
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

### Naming overrides

variable "helm_release_name_override" {
  description = "If set, overrides the name of the aptos-node helm chart"
  default     = ""
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

### GKE cluster config

variable "cluster_ipv4_cidr_block" {
  description = "The IP address range of the container pods in this cluster, in CIDR notation. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#cluster_ipv4_cidr_block"
  default     = ""
}

### Helm

variable "num_validators" {
  description = "The number of validator nodes to create"
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
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
