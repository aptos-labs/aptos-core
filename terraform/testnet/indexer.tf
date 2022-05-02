module "indexer" {
  source = "../modules/indexer"

  count  = var.enable_indexer ? 1 : 0
  region = var.region

  image_tag = var.image_tag

  # This is the default API service created by testnet helm chart
  node_url = "http://aptos-testnet-api:80"

  oidc_provider = module.validator.oidc_provider

  subnet_ids = var.indexer_db_publicly_accessible ? module.validator.aws_subnet_public.*.id : module.validator.aws_subnet_private.*.id
  vpc_id     = module.validator.vpc_id

  db_password            = var.indexer_db_password
  db_publicly_accessible = var.indexer_db_publicly_accessible

  indexer_helm_values = var.indexer_helm_values
}
