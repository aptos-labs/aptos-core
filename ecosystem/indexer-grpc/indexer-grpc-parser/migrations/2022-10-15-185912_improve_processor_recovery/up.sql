-- Your SQL goes here
-- Tracks latest processed version per processor
CREATE TABLE processor_status (
  processor VARCHAR(50) UNIQUE PRIMARY KEY NOT NULL,
  last_success_version BIGINT NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW()
);