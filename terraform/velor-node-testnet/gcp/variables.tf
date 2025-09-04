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
  default     = ""
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

### Chain config

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

variable "image_tag" {
  description = "Docker image tag for Velor node"
  type        = string
  default     = "devnet"
}

### DNS config

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

variable "record_name" {
  description = "DNS record name to use (<workspace> is replaced with the TF workspace name)"
  type        = string
  default     = "<workspace>.velor"
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

### Testnet config

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  type        = string
  default     = ""
}

variable "helm_release_name_override" {
  description = "If set, overrides the name of the velor-node helm chart"
  type        = string
  default     = ""
}

variable "velor_node_helm_values" {
  description = "Map of values to pass to velor-node helm chart"
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
  type        = number
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  type        = number
  default     = 1
}


### K8s config

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

### Addons

variable "enable_forge" {
  description = "Enable Forge"
  type        = bool
  default     = false
}

variable "enable_genesis" {
  description = "Perform genesis automatically"
  type        = bool
  default     = true
}

variable "testnet_addons_helm_values" {
  description = "Map of values to pass to testnet-addons helm chart"
  type        = any
  default     = {}
}

### Node pools and Autoscaling

variable "default_disk_size_gb" {
  description = "Default disk size for nodes"
  type        = number
  default     = 200
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

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  type        = string
  default     = "t2d-standard-60"
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint instances in the utilities nodegroup"
  type        = bool
  default     = true
}

variable "validator_instance_enable_taint" {
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
  description = "Maximum CPU allocation for GKE node_autoprovisioning"
  type        = number
  default     = 500
}

variable "gke_node_autoprovisioning_max_memory" {
  description = "Maximum memory allocation for GKE node_autoprovisioning"
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

variable "enable_vertical_pod_autoscaling" {
  description = "Enable vertical pod autoscaling"
  type        = bool
  default     = false
}

### GKE cluster config

variable "cluster_ipv4_cidr_block" {
  description = "The IP address range of the container pods in this cluster, in CIDR notation. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#cluster_ipv4_cidr_block"
  type        = string
  default     = ""
}

variable "router_nat_ip_allocate_option" {
  description = "The method of NAT IP allocation for the cluster. See https://registry.terraform.io/providers/hashicorp/google/latest/docs/resources/container_cluster#router_nat_ip_allocate_option"
  type        = string
  default     = "MANUAL_ONLY"
}

variable "enable_endpoint_independent_mapping" {
  description = "Enable endpoint independent mapping for the NAT router"
  type        = bool
  default     = true
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
