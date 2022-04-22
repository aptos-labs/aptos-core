# Aptos Indexer

Aptos Indexer helm chart simply installs the aptos indexer with database connection URI stored in `indexer_credentials` secret.

Since the DB and indexer may exist in a private network, developers can utilize the provided nginx to port-foward locally the DB connection. For example:

```
# ensure nginx.enabled=true
$ kubectl port-forward deployment/indexer-nginx 5432:5432
```

Then, you may connect to `localhost:5432` using the configured username and password in the upstream database. For example, you could use `psql` or even Postico.
