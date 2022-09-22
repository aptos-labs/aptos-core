variable "region" {
  description = "AWS region"
  type        = string
}

variable "num_azs" {
  description = "Number of availability zones"
  default     = 3
}

variable "kubernetes_version" {
  description = "Version of Kubernetes to use for EKS cluster"
  default     = "1.22"
}

variable "k8s_api_sources" {
  description = "List of CIDR subnets which can access the Kubernetes API endpoint"
  default     = ["0.0.0.0/0"]
}

variable "num_validators" {
  description = "The number of validator nodes to create"
  default     = 1
}

variable "num_fullnode_groups" {
  description = "The number of fullnode groups to create"
  default     = 1
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

variable "zone_id" {
  description = "Zone ID of Route 53 domain to create records in"
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

variable "create_records" {
  description = "Creates DNS records in var.zone_id that point to k8s service, as opposed to using external-dns or other means"
  default     = true
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

variable "k8s_admins" {
  description = "List of AWS usernames to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_admin_roles" {
  description = "List of AWS roles to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_viewers" {
  description = "List of AWS usernames to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_viewer_roles" {
  description = "List of AWS roles to configure as Kubernetes viewers"
  type        = list(string)
  default     = []
}

variable "k8s_debuggers" {
  description = "List of AWS usernames to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "k8s_debugger_roles" {
  description = "List of AWS roles to configure as Kubernetes debuggers"
  type        = list(string)
  default     = []
}

variable "iam_path" {
  default     = "/"
  description = "Path to use when naming IAM objects"
}

variable "permissions_boundary_policy" {
  default     = ""
  description = "ARN of IAM policy to set as permissions boundary on created roles"
}

variable "vpc_cidr_block" {
  default     = "192.168.0.0/16"
  description = "VPC CIDR Block"
}

variable "maximize_single_az_capacity" {
  description = "Whether to maximize the capacity of the cluster by allocating more IPs to the first AZ"
  default     = false
}

variable "helm_enable_validator" {
  description = "Enable deployment of the validator Helm chart"
  default     = true
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.2xlarge"
}

variable "utility_instance_num" {
  description = "Number of instances for utilities"
  default     = 1
}

variable "utility_instance_min_num" {
  description = "Minimum number of instances for utilities"
  default     = 1
}

variable "utility_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.utility_instance_num"
  default     = 0
}

variable "utility_instance_enable_taint" {
  description = "Whether to taint the instances in the utility nodegroup"
  default     = false
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c6i.4xlarge"
}

variable "validator_instance_num" {
  description = "Number of instances used for validator and fullnodes"
  default     = 2
}

variable "validator_instance_min_num" {
  description = "Minimum number of instances for validators"
  default     = 1
}

variable "validator_instance_max_num" {
  description = "Maximum number of instances for utilities. If left 0, defaults to 2 * var.validator_instance_num"
  default     = 0
}

variable "validator_instance_enable_taint" {
  description = "Whether to taint instances in the validator nodegroup"
  default     = false
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "enable_calico" {
  description = "Enable Calico networking for NetworkPolicy"
  default     = true
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

variable "enable_prometheus_node_exporter" {
  description = "Enable prometheus-node-exporter within monitoring helm chart"
  default     = false
}

variable "enable_kube_state_metrics" {
  description = "Enable kube-state-metrics within monitoring helm chart"
  default     = false
}

variable "helm_release_name_override" {
  description = "If set, overrides the name of the aptos-node helm chart"
  default     = ""
}
