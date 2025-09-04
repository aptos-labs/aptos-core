/*
region = "us-central1"  # Specify the region
zone = "c"  # Specify the zone suffix
project = ""  # Specify your GCP project name

# Example fullnode helm values
fullnode_helm_values = {
  velor_chains = {
    devnet = {
      seeds = {
        "7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61" = {
        addresses = ["/dns4/pfn1.node.devnet.velorlabs.com/tcp/6182/noise-ik/7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61/handshake/0"]
        role = "Upstream"
        }
      }
    }
  }
}
*/