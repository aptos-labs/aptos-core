resource "random_password" "vault-root" {
  length  = 24
  special = false
}

resource "null_resource" "vault-init" {
  provisioner "local-exec" {
    command = <<-EOT
      export VAULT_TOKEN="$(vault operator init | grep 'Root Token' | cut -d: -f2 | tr -d ' ')" &&
      sleep 1 &&
      vault token create -id=${random_password.vault-root.result} -policy=root
    EOT

    environment = {
      VAULT_ADDR   = module.validator.vault.server.address
      VAULT_CACERT = "${terraform.workspace}-vault.ca"
    }
  }
}

provider "vault" {
  address      = module.validator.vault.server.address
  ca_cert_file = "${terraform.workspace}-vault.ca"
  token        = random_password.vault-root.result
  namespace    = null_resource.vault-init.id
}

resource "vault_mount" "secret" {
  path = "secret"
  type = "kv-v2"
}

resource "vault_mount" "transit" {
  path = "transit"
  type = "transit"
}

module "vault" {
  count  = var.num_validators
  source = "../validator/vault-init"
  providers = {
    vault = vault
  }

  mount_engines     = false
  reset_safety_data = false
  namespace         = "val${count.index}"

  kubernetes_host        = module.validator.kubernetes.kubernetes_host
  kubernetes_ca_cert     = module.validator.kubernetes.kubernetes_ca_cert
  issuer                 = module.validator.kubernetes.issuer
  service_account_prefix = "val${count.index}-aptos-validator"

  depends_on_ = [vault_mount.secret.accessor, vault_mount.transit.accessor]
}

resource "vault_transit_secret_backend_key" "aptos_root" {
  backend          = vault_mount.transit.path
  name             = "aptos__aptos_root"
  type             = "ed25519"
  deletion_allowed = true
  exportable       = true
}

data "vault_policy_document" "genesis-root" {
  rule {
    path         = "${vault_mount.transit.path}/keys/${vault_transit_secret_backend_key.aptos_root.name}"
    capabilities = ["read"]
    description  = "Allow reading the Aptos root public key"
  }
  rule {
    path         = "${vault_mount.transit.path}/export/signing-key/${vault_transit_secret_backend_key.aptos_root.name}"
    capabilities = ["read"]
    description  = "Allow reading the Aptos root private key"
  }

}

resource "vault_policy" "genesis-root" {
  name   = "genesis-root"
  policy = data.vault_policy_document.genesis-root.hcl
}

resource "vault_kubernetes_auth_backend_role" "genesis" {
  backend                          = module.vault[0].kubernetes_auth_path
  role_name                        = "genesis"
  bound_service_account_names      = ["${helm_release.testnet.name}-testnet"]
  bound_service_account_namespaces = ["*"]
  token_policies                   = concat([vault_policy.genesis-root.name], formatlist("val%s-management", range(var.num_validators)))
}

data "vault_policy_document" "genesis-reset" {
  rule {
    path         = "${vault_mount.secret.path}/data/*"
    capabilities = ["update"]
    description  = "Allow updating validator secrets"
  }
  rule {
    path         = "${vault_mount.transit.path}/keys/+/rotate"
    capabilities = ["update"]
    description  = "Allow rotating keys"
  }
}

resource "vault_policy" "genesis-reset" {
  name   = "genesis-reset"
  policy = data.vault_policy_document.genesis-reset.hcl
}

resource "vault_auth_backend" "approle" {
  type  = "approle"
}

resource "vault_approle_auth_backend_role" "genesis-reset-role" {
  backend        = vault_auth_backend.approle.path
  role_name      = "genesis-reset-role"
  token_policies = [vault_policy.genesis-reset.name]
}

resource "vault_approle_auth_backend_role_secret_id" "genesis-reset-id" {
  backend   = vault_auth_backend.approle.path
  role_name = vault_approle_auth_backend_role.genesis-reset-role.role_name
}
