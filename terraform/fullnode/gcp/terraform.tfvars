/*
region = "us-central1"  # Specify the region
zone = "c"  # Specify the zone suffix
project = ""  # Specify your GCP project name

# Example fullnode helm values
fullnode_helm_values = {
  aptos_chains = {
    devnet = {
      seeds = {
        "5dfa8623d0020eb7c74bb4f74e853079" = {
        addresses = ["/ip4/135.181.103.127/tcp/6180/ln-noise-ik/d894cded087f19b567648ffefa4277ab5dfa8623d0020eb7c74bb4f74e853079/ln-handshake/0"]
        role = "Upstream"
        }
      }
    }
  }
}
*/