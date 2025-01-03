# Indexer GRPC file checker
A program that compares files in two buckets and to make sure the content are the same.

## How to run it.

Example of config: 

```
health_check_port: 8081
    server_config:
      existing_bucket_name: bucket_being_used
      new_bucket_name: bucket_with_new_sharding
      starting_version: 123123
```
