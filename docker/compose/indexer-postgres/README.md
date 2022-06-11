# Aptos-Indexer, Postgres and pgAdmin panel

Docker Compose file provides Postgres database, pgAdmin panel and Aptos Indexer as single deployment.

## Setup

Create `.env` file and populate it with variables from `env.example`

`NODE_URL` URL and port to your node, e.g. `NODE_URL=http://yournode:8080`

Then, start it with the following command:

```shell
docker compose up -d
```

Confirm no errors and indexing has started by:

```shell
docker logs --follow aptos-indexer
```

## Configure pgAdmin panel

Login to `http://localhost:5050` with `PGADMIN_EMAIL` and
`PGADMIN_PASSWORD` from `.env` file and **Add New Server**.
In **General** section tab fill in **Server Name**, then go to **Connection** tab and specify postgres hostname as `postgres.local` with default port `5432`. Change **Maintenance database** to `indexer` and fill in `POSTGRES_USER` and `POSTGRES_PASSWORD` from `.env` file.
