module "indexer" {
  source = "../../modules/indexer"

  count  = var.enable_indexer ? 1 : 0
  region = var.region

  image_tag = var.image_tag

  # This is the default API service created by testnet helm chart
  node_url = "http://aptos-testnet-api:80"

  oidc_provider = module.eks.oidc_provider

  subnet_ids = var.indexer_db_publicly_accessible ? module.eks.aws_subnet_public.*.id : module.eks.aws_subnet_private.*.id
  vpc_id     = module.eks.vpc_id

  db_password            = var.indexer_db_password
  db_publicly_accessible = var.indexer_db_publicly_accessible

  indexer_helm_values = var.indexer_helm_values
}
