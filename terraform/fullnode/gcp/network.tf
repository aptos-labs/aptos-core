resource "google_compute_network" "velor" {
  name                    = "velor-${terraform.workspace}"
  auto_create_subnetworks = true
}

# If the google_compute_subnetwork data source resolves immediately after the
# network is created, it doesn't find the subnet and returns null. This results
# in the vault-lb address being created in the default network.
resource "time_sleep" "create-subnetworks" {
  create_duration = "30s"
  depends_on      = [google_compute_network.velor]
}

data "google_compute_subnetwork" "region" {
  name       = google_compute_network.velor.name
  depends_on = [time_sleep.create-subnetworks]
}

resource "google_compute_router" "nat" {
  name    = "velor-${terraform.workspace}-nat"
  network = google_compute_network.velor.id
}

resource "google_compute_address" "nat" {
  name = "velor-${terraform.workspace}-nat"
}

resource "google_compute_router_nat" "nat" {
  name                                = "velor-${terraform.workspace}-nat"
  router                              = google_compute_router.nat.name
  nat_ip_allocate_option              = var.router_nat_ip_allocate_option
  nat_ips                             = var.router_nat_ip_allocate_option == "MANUAL_ONLY" ? [google_compute_address.nat.self_link] : null
  source_subnetwork_ip_ranges_to_nat  = "ALL_SUBNETWORKS_ALL_PRIMARY_IP_RANGES"
  min_ports_per_vm                    = var.router_nat_ip_allocate_option == "MANUAL_ONLY" ? null : 32
  enable_endpoint_independent_mapping = var.enable_endpoint_independent_mapping
  # EndpointIndependentMapping and DynamicPortAllocation are mutually exclusive.
  enable_dynamic_port_allocation = !var.enable_endpoint_independent_mapping
}
