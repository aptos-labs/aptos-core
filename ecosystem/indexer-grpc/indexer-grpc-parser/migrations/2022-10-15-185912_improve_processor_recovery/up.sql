-- Your SQL goes here
-- Tracks latest processed version per processor
CREATE TABLE processor_status (
  processor VARCHAR(50) PRIMARY KEY NOT NULL,
  last_success_version BIGINT NOT NULL,
  last_updated BIGINT NOT NULL
);
CREATE UNIQUE INDEX ps_p_index ON processor_status (processor);