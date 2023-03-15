-- Your SQL goes here
-- manually toggle indexer status on/off
CREATE TABLE indexer_status (
  db VARCHAR(50) PRIMARY KEY NOT NULL,
  is_indexer_up BOOLEAN NOT NULL
);
CREATE UNIQUE INDEX is_d_index ON indexer_status (db);