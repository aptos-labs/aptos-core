# Typescript Indexer Interface

> A Typescript interface for querying the Aptos Postgres Indexer DB

This is an express.js based REST client that interacts with [Prisma](https://www.prisma.io/) as an ORM for querying
the Aptos Postgres Indexer DB.

## Getting started

### 1. [Complete the Local Development Guide for the Rust Indexer](../rust/README.md) (Highly Reccomended)
This will make sure you have the following setup on your local:
1. postgres
2. diesel cli

### 2. Get Local Development setup for Typescript
1. Ensure you have `npm`, `yarn`, and `node` installed via homebrew (recommended through a node package manager like [`nvm`](https://formulae.brew.sh/formula/nvm))
2. `yarn install`
3. create a `.env` file at the base of `indexer > typescript`
4. Add the `Prisma` VSCode extension (the most popular one) for syntax highlighting and formatting

`.env` file contents should look something like this
```bash
# See https://stackoverflow.com/questions/3582552/what-is-the-format-for-the-postgresql-connection-string-url
DATABASE_URL=postgresql://postgres@localhost/postgres
```
4. Ensure your postgres server is up and running
5. `yarn run generate`
6. `yarn run dev`

## Using Prisma
We are using the Rust -> Diesel models as a source of truth for the database schema.
The database schema that Prisma uses is defined in the `indexer > typescript > prisma > schema.prisma` file.


### Updating DB Schema
When updating the schema please do the following one way loop:
1. Make your changes to the data models in `indexer > rust > models`
2. Run a db migration
3. Pull db migration changes into prisma via `npx prisma db pull`
4. Run `yarn run generate:prisma`

You should never have to modify the `schema.prisma` file manually.

## Next steps

- Check out the [Prisma docs](https://www.prisma.io/docs)
