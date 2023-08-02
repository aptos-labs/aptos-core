# NFT Metadata Crawler Parser

To run the NFT Metadata Crawler Parser:

1. Set up GCP project with PubSub, GCS (set to public with Load Balancer + Cloud CDN), and a service account with access to both
2. Set up Postgres database
3. Create yaml config file with the following format, template file is provided:

```yaml
health_check_port: 8080
server_config:
    google_application_credentials: PATH_TO_CREDENTIALS_FILE
    bucket: BUCKET_NAME
    subscription_name: projects/PROJECT_NAME/subscriptions/SUBSCRIPTION_NAME
    database_url: postgres://localhost:5432/postgres
    cdn_prefix: https://YOUR_CDN_URI.com
    ipfs_prefix: https://YOUR_IPFS_PREFIX.com
    num_parsers: 10
    max_file_size_bytes: 50000000
    image_quality: 50
```

4. Install diesel and run database migrations: 
```bash
cargo install diesel_cli --no-default-features --features postgres && diesel migration run
```

If receiving error regarding `DATABASE_URL` not being set, run the following command:
```bash
export DATABASE_URL=POSTGRES_URL
```

5. Build the binary:
```bash
cargo build --release
```

6. Locate the binary (most likely in `target/release/aptos-nft-metadata-crawler-parser`) and run:
```bash
PATH_TO_BINARY --config-path PATH_TO_CONFIG
```
