variable "region" {
  description = "AWS region"
}

variable "iam_path" {
  default     = "/"
  description = "Path to use when naming IAM objects"
}

variable "permissions_boundary_policy" {
  default     = ""
  description = "ARN of IAM policy to set as permissions boundary on created roles"
}

variable "admin_sources_ipv4" {
  description = "List of CIDR subnets which can access Kubernetes API"
  type        = list(string)
}

variable "client_sources_ipv4" {
  description = "List of CIDR subnets which can access the testnet API"
  type        = list(string)
}

variable "k8s_admin_roles" {
  description = "List of AWS roles to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "k8s_admins" {
  description = "List of AWS usernames to configure as Kubernetes administrators"
  type        = list(string)
  default     = []
}

variable "ssh_pub_key" {
  description = "SSH public key to configure for bastion and vault access"
}

variable "num_validators" {
  default = 4
}

variable "num_public_fullnodes" {
  default = 1
}

variable "image_tag" {
  description = "Docker image tag for aptos components. Overrides ecr_repo method."
  default     = ""
}

variable "ecr_repo" {
  description = "Name of an ECR repo to resolve 'stable' tag to a specific revision"
  default     = ""
}

variable "era" {
  description = "Chain era, used to start a clean chain"
  default     = 15
}

variable "chain_id" {
  description = "aptos chain ID"
  default     = "DEVNET"
}

variable "validator_helm_values" {
  description = "Map of values to pass to validator Helm"
  type        = any
  default     = {}
}

variable "testnet_helm_values" {
  description = "Map of values to pass to testnet Helm"
  type        = any
  default     = {}
}

variable "public_fullnode_helm_values" {
  description = "Map of values to pass to public fullnode Helm"
  type        = any
  default     = {}
}

variable "zone_id" {
  description = "Route53 Zone ID to create records in"
  default     = ""
}

variable "tls_sans" {
  description = "List of Subject Alternate Names to include in TLS certificate"
  type        = list(string)
  default     = []
}

variable "workspace_dns" {
  description = "Include Terraform workspace name in DNS records"
  default     = true
}

variable "enable_pfn_logger" {
  description = "Enable separate public fullnode logger pod"
  default     = false
}

variable "pfn_logger_helm_values" {
  description = "Map of values to pass to public fullnode logger Helm"
  type        = any
  default     = {}
}

variable "enable_explorer" {
  description = "Enable Aptos Explorer on the cluster"
  default     = false
}

variable "enable_forge" {
  description = "Enable Forge test framework, also creating an internal helm repo"
  default     = false
}

variable "forge_helm_values" {
  description = "Map of values to pass to Forge Helm"
  type        = any
  default     = {}
}

variable "utility_instance_type" {
  description = "Instance type used for utilities"
  default     = "t3.medium"
}

variable "validator_instance_type" {
  description = "Instance type used for validator and fullnodes"
  default     = "c5.xlarge"
}

variable "trusted_instance_type" {
  description = "Instance type used for trusted components"
  default     = "c5.large"
}

variable "explorer_image_repo" {
  default = "ghcr.io/aptos/explorer"
}

variable "explorer_image_tag" {
  default = "latest"
}

variable "enable_api" {
  description = "Enables the REST API on fullnodes"
  default     = false
}

variable "enable_dev_vault" {
  description = "TEST ONLY: Enables Vault in Dev Mode for all validators"
  default     = false
}
