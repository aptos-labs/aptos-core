locals {
  vault = {
    server = {
      address = "https://${azurerm_lb.vault.private_ip_address}:8200"
      ca_cert = "/etc/vault/ca.crt"
    }
    tls = {
      "ca.crt" = tls_self_signed_cert.ca.cert_pem
    }
    prometheusTarget = "${azurerm_lb.vault.private_ip_address}:8200"
    serverIPRanges   = ["${azurerm_lb.vault.private_ip_address}/32"]
  }
}
