locals {
  vault = {
    server = {
      address = "https://${google_compute_forwarding_rule.vault.ip_address}:8200"
      ca_cert = "/etc/vault/ca.crt"
    }
    tls = {
      "ca.crt" = tls_self_signed_cert.ca.cert_pem
    }
    prometheusTarget = "${google_compute_forwarding_rule.vault.ip_address}:8200"
    serverIPRanges   = ["${google_compute_forwarding_rule.vault.ip_address}/32"]
  }
}
