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
  default     = "" # if empty, it's a regional cluster
}

variable "node_locations" {
  description = "List of node locations"
  type        = list(string)
  default     = [] # if empty, let GCP choose
}

variable "manage_via_tf" {
  description = "Whether to manage the velor-node k8s workload via Terraform. If set to false, the helm_release resource will still be created and updated when values change, but it may not be updated on every apply"
  type        = bool
  default     = true
}

### DNS

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  type        = bool
  default     = true
}

variable "dns_prefix_name" {
  description = "DNS prefix for fullnode url"
  type        = string
  default     = "fullnode"
}

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

variable "create_google_managed_ssl_certificate" {
  description = "Whether to create a Google Managed SSL Certificate for the GCE Ingress"
  type        = bool
  default     = false
}

variable "backend_http2" {
  description = "Whether to enable HTTP/2 between Ingress and backends"
  type        = bool
  default     = false
}

### Node pools and Autoscaling

variable "node_pool_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the number of nodes in the specified pool"
}

variable "instance_disk_sizes" {
  type        = map(number)
  default     = {}
  description = "Override the disk size in the specified pool"
}

variable "default_disk_size_gb" {
  description = "Default disk size for nodes"
  type        = number
  default     = 100
}

variable "default_disk_type" {
  description = "Default disk type for nodes"
  type        = string
  default     = "pd-standard"
}

variable "create_nodepools" {
  description = "Create managed nodepools"
  type        = bool
  default     = true
}

variable "nodepool_sysctls" {
  description = "Sysctls to set on nodepools"
  type        = map(string)
  default     = {}
}

variable "core_instance_type" {
  description = "Instance type used for core pods"
  type        = string
  default     = "e2-medium"
}

variable "utility_instance_type" {
  description = "Instance type used for utility pods"
  type        = string
  default     = "e2-standard-8"
}

variable "fullnode_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "t2d-standard-60"
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint instances in the utilities nodegroup"
  type        = bool
  default     = false
}

variable "fullnode_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  type        = bool
  default     = true
}

variable "gke_enable_node_autoprovisioning" {
  description = "Enable GKE node autoprovisioning"
  type        = bool
  default     = true
}

variable "gke_node_autoprovisioning_max_cpu" {
  description = "Maximum CPU allocation for GKE node autoprovisioning"
  type        = number
  default     = 500
}

variable "gke_node_autoprovisioning_max_memory" {
  description = "Maximum memory allocation for GKE node autoprovisioning"
  type        = number
  default     = 2000
}

variable "gke_autoscaling_profile" {
  description = "Autoscaling profile for GKE cluster. See https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-autoscaler#autoscaling_profiles"
  type        = string
  default     = "OPTIMIZE_UTILIZATION"
}

variable "gke_autoscaling_max_node_count" {
  description = "Maximum number of nodes for GKE nodepool autoscaling"
  type        = number
  default     = 250
}

### Naming overrides

variable "helm_release_name_override" {
  description = "If set, overrides the name of the velor-node helm chart"
  type        = string
  default     = ""
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

### GKE cluster config

variable "router_nat_ip_allocate_option" {
  description = "The method of NAT IP allocation for the cluster. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#router_nat_ip_allocate_option"
  type        = string
  default     = "MANUAL_ONLY"
}

variable "enable_endpoint_independent_mapping" {
  description = "Enable endpoint independent mapping for the NAT router"
  type        = bool
  default     = false
}

variable "enable_clouddns" {
  description = "Enable CloudDNS (Google-managed cluster DNS)"
  type        = bool
  default     = false
}

variable "enable_image_streaming" {
  description = "Enable image streaming (GCFS)"
  type        = bool
  default     = false
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

### Helm

variable "helm_values" {
  description = "Map of values to pass to Helm"
  type        = any
  default     = {}
}

variable "pfn_helm_values" {
  description = "Map of values to pass to pfn-addons Helm"
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

variable "fullnode_name" {
  description = "Name of the fullnode node owner"
  type        = string
}

### Addons

variable "enable_backup" {
  description = "Enable data backup from fullnode"
  type        = bool
  default     = false
}

variable "enable_public_backup" {
  description = "Provide data backups to the public"
  type        = bool
  default     = false
}

variable "backup_fullnode_index" {
  description = "Index of fullnode to backup data from"
  type        = number
  default     = 0
}

variable "tls_sans" {
  description = "List of Subject Alternate Names to include in TLS certificate"
  type        = list(string)
  default     = []
}

variable "enable_monitoring" {
  description = "Enable monitoring helm chart"
  type        = bool
  default     = false
}

variable "monitoring_helm_values" {
  description = "Map of values to pass to monitoring Helm"
  type        = any
  default     = {}
}

variable "enable_prometheus_node_exporter" {
  description = "Enable prometheus-node-exporter within monitoring helm chart"
  type        = bool
  default     = false
}

variable "enable_kube_state_metrics" {
  description = "Enable kube-state-metrics within monitoring helm chart"
  type        = bool
  default     = false
}
