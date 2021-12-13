data "vault_policy_document" "safety-rules" {
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/*"
    capabilities = ["read", "update", "create"]
    description  = "Allow read and write on safety-rules secure data"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}"
    capabilities = ["read"]
    description  = "Allow reading the consensus public key"
  }
  rule {
    path         = "${var.transit_mount}/export/signing-key/${vault_transit_secret_backend_key.consensus.name}"
    capabilities = ["read"]
    description  = "Allow reading the consensus private key"
  }
  rule {
    path         = "${var.transit_mount}/sign/${vault_transit_secret_backend_key.consensus.name}"
    capabilities = ["update"]
    description  = "Allow signing with the consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.execution.name}"
    capabilities = ["read"]
    description  = "Allow reading the execution public key"
  }
}

resource "vault_policy" "safety-rules" {
  name   = "${var.namespace}-safety-rules"
  policy = data.vault_policy_document.safety-rules.hcl
}

data "vault_policy_document" "validator" {
  rule {
    path         = "${var.transit_mount}/export/signing-key/${vault_transit_secret_backend_key.execution.name}"
    capabilities = ["read"]
    description  = "Allow reading the execution private key"
  }
  rule {
    path         = "${var.transit_mount}/export/signing-key/${vault_transit_secret_backend_key.validator_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the validator_network private key"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/owner_account"
    capabilities = ["read"]
    description  = "Allow reading the owner account"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/genesis-waypoint"
    capabilities = ["read"]
    description  = "Allow reading the genesis waypoint"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/validator_network_address_keys"
    capabilities = ["read"]
    description  = "Allow reading the shared validator network address keys"
  }
}

resource "vault_policy" "validator" {
  name   = "${var.namespace}-validator"
  policy = data.vault_policy_document.validator.hcl
}

data "vault_policy_document" "fullnode" {
  rule {
    path         = "${var.transit_mount}/export/signing-key/${vault_transit_secret_backend_key.fullnode_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the fullnode_network private key"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/owner_account"
    capabilities = ["read"]
    description  = "Allow reading the owner account"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/genesis-waypoint"
    capabilities = ["read"]
    description  = "Allow reading the genesis waypoint"
  }
}

resource "vault_policy" "fullnode" {
  name   = "${var.namespace}-fullnode"
  policy = data.vault_policy_document.fullnode.hcl
}

data "vault_policy_document" "key-manager" {
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/*"
    capabilities = ["read"]
    description  = "Allow reading safety-rules secure data"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.operator.name}"
    capabilities = ["read"]
    description  = "Allow reading the operator public key"
  }
  rule {
    path         = "${var.transit_mount}/sign/${vault_transit_secret_backend_key.operator.name}"
    capabilities = ["update"]
    description  = "Allow signing with the operator key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}"
    capabilities = ["read"]
    description  = "Allow reading the consensus public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/config"
    capabilities = ["update"]
    allowed_parameter {
      key   = "min_decryption_version"
      value = []
    }
    allowed_parameter {
      key   = "min_encryption_version"
      value = []
    }
    description = "Allow setting minimum versions of consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/trim"
    capabilities = ["update"]
    description  = "Allow trimming the consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the validator_network public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the validator_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the fullnode_network public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the fullnode_network key"
  }
}

resource "vault_policy" "key-manager" {
  name   = "${var.namespace}-key-manager"
  policy = data.vault_policy_document.key-manager.hcl
}

data "vault_policy_document" "management" {
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/*"
    capabilities = ["read"]
    description  = "Allow reading safety-rules secure data"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/waypoint"
    capabilities = ["read", "update", "create"]
    description  = "Allow reading and updating the waypoint"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/genesis-waypoint"
    capabilities = ["read", "update", "create"]
    description  = "Allow reading and updating the genesis waypoint"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/owner_account"
    capabilities = ["read", "update", "create"]
    description  = "Allow reading and updating the owner account"
  }
  rule {
    path         = "${var.kv_v2_mount}/data/${var.namespace}/operator_account"
    capabilities = ["read", "update", "create"]
    description  = "Allow reading and updating the operator account"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.owner.name}"
    capabilities = ["read"]
    description  = "Allow reading the owner public key"
  }
  rule {
    path         = "${var.transit_mount}/sign/${vault_transit_secret_backend_key.owner.name}"
    capabilities = ["update"]
    description  = "Allow signing with the owner key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.operator.name}"
    capabilities = ["read"]
    description  = "Allow reading the operator public key"
  }
  rule {
    path         = "${var.transit_mount}/sign/${vault_transit_secret_backend_key.operator.name}"
    capabilities = ["update"]
    description  = "Allow signing with the operator key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.operator.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the operator key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}"
    capabilities = ["read"]
    description  = "Allow reading the consensus public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/config"
    capabilities = ["update"]
    allowed_parameter {
      key   = "min_decryption_version"
      value = []
    }
    allowed_parameter {
      key   = "min_encryption_version"
      value = []
    }
    description = "Allow setting minimum versions of consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.consensus.name}/trim"
    capabilities = ["update"]
    description  = "Allow trimming the consensus key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the validator_network public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the validator_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}/config"
    capabilities = ["update"]
    allowed_parameter {
      key   = "min_decryption_version"
      value = []
    }
    allowed_parameter {
      key   = "min_encryption_version"
      value = []
    }
    description = "Allow setting minimum versions of validator_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.validator_network.name}/trim"
    capabilities = ["update"]
    description  = "Allow trimming the validator_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}"
    capabilities = ["read"]
    description  = "Allow reading the fullnode_network public key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}/rotate"
    capabilities = ["update"]
    description  = "Allow rotating the fullnode_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}/config"
    capabilities = ["update"]
    allowed_parameter {
      key   = "min_decryption_version"
      value = []
    }
    allowed_parameter {
      key   = "min_encryption_version"
      value = []
    }
    description = "Allow setting minimum versions of fullnode_network key"
  }
  rule {
    path         = "${var.transit_mount}/keys/${vault_transit_secret_backend_key.fullnode_network.name}/trim"
    capabilities = ["update"]
    description  = "Allow trimming the fullnode_network key"
  }
}

resource "vault_policy" "management" {
  name   = "${var.namespace}-management"
  policy = data.vault_policy_document.management.hcl
}

resource "vault_token_auth_backend_role" "management" {
  role_name              = "${var.namespace}-management"
  allowed_policies       = [vault_policy.management.name]
  renewable              = false
  token_explicit_max_ttl = 43200 # 12h
}
