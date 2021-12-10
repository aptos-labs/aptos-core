locals {
  vault = {
    server = {
      address = "https://${aws_lb.vault.dns_name}:8200"
      ca_cert = "/etc/vault/ca.crt"
    }
    tls = {
      "ca.crt" = tls_self_signed_cert.ca.cert_pem
    }
    prometheusTarget = "${aws_lb.vault.dns_name}:8200"
    serverIPRanges   = aws_subnet.private.*.cidr_block
  }
}
