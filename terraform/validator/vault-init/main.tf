provider "vault" {}

variable "namespace" {
  description = "Prefix to use when naming secrets and transit keys"
  default     = "diem"
}

variable "kv_v2_mount" {
  description = "Mount path of a Key/Value version 2 engine"
  default     = "secret"
}

variable "transit_mount" {
  description = "Mount path of a Transit engine"
  default     = "transit"
}

variable "mount_engines" {
  description = "Create the KV-v2 and Transit engine mounts"
  default     = true
}

variable "reset_safety_data" {
  description = "Reset the Diem Safety Rules counters when applying"
  default     = true
}

variable "validator_network_address_key" {
  description = "Decryption key for validator network address"
  type        = string
}

resource "vault_mount" "secret" {
  count = var.mount_engines ? 1 : 0
  path  = var.kv_v2_mount
  type  = "kv-v2"
}

resource "vault_mount" "transit" {
  count = var.mount_engines ? 1 : 0
  path  = var.transit_mount
  type  = "transit"
}

resource "null_resource" "mounts_created" {
  triggers = {
    kv_v2   = join("", vault_mount.secret[*].accessor)
    transit = join("", vault_mount.transit[*].accessor)
  }
}

resource "vault_generic_secret" "safety_data" {
  path = "${var.kv_v2_mount}/${var.namespace}/safety_data"
  data_json = jsonencode({
    safety_data = {
      epoch            = 0
      last_voted_round = 0
      preferred_round  = 0
      last_vote        = null
    }
  })
  disable_read = ! var.reset_safety_data
  depends_on   = [null_resource.mounts_created]
}

resource "vault_generic_secret" "owner_account" {
  path         = "${var.kv_v2_mount}/${var.namespace}/owner_account"
  data_json    = "{}"
  depends_on   = [null_resource.mounts_created]
  disable_read = true
}

resource "vault_generic_secret" "operator_account" {
  path         = "${var.kv_v2_mount}/${var.namespace}/operator_account"
  data_json    = "{}"
  depends_on   = [null_resource.mounts_created]
  disable_read = true
}

resource "vault_generic_secret" "validator_network_address_keys" {
  path = "${var.kv_v2_mount}/${var.namespace}/validator_network_address_keys"
  data_json = jsonencode({
    validator_network_address_keys = {
      current = 0
      keys = {
        "0" = var.validator_network_address_key
      }
    }
  })
  depends_on = [null_resource.mounts_created]
}

resource "vault_transit_secret_backend_key" "owner" {
  backend    = var.transit_mount
  name       = "${var.namespace}__owner"
  type       = "ed25519"
  depends_on = [null_resource.mounts_created]
}

resource "vault_transit_secret_backend_key" "operator" {
  backend    = var.transit_mount
  name       = "${var.namespace}__operator"
  type       = "ed25519"
  depends_on = [null_resource.mounts_created]
  lifecycle {
    ignore_changes = [min_decryption_version, min_encryption_version]
  }
}

resource "vault_transit_secret_backend_key" "consensus" {
  backend    = var.transit_mount
  name       = "${var.namespace}__consensus"
  type       = "ed25519"
  depends_on = [null_resource.mounts_created]
  lifecycle {
    ignore_changes = [min_decryption_version, min_encryption_version]
  }
}

resource "vault_transit_secret_backend_key" "execution" {
  backend    = var.transit_mount
  name       = "${var.namespace}__execution"
  type       = "ed25519"
  exportable = true
  depends_on = [null_resource.mounts_created]
  lifecycle {
    ignore_changes = [min_decryption_version, min_encryption_version]
  }
}

resource "vault_transit_secret_backend_key" "validator_network" {
  backend    = var.transit_mount
  name       = "${var.namespace}__validator_network"
  type       = "ed25519"
  exportable = true
  depends_on = [null_resource.mounts_created]
  lifecycle {
    ignore_changes = [min_decryption_version, min_encryption_version]
  }
}

resource "vault_transit_secret_backend_key" "fullnode_network" {
  backend    = var.transit_mount
  name       = "${var.namespace}__fullnode_network"
  type       = "ed25519"
  exportable = true
  depends_on = [null_resource.mounts_created]
  lifecycle {
    ignore_changes = [min_decryption_version, min_encryption_version]
  }
}
