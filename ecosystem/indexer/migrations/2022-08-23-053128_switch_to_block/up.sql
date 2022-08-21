CREATE TABLE v2_processor_statuses (
  name VARCHAR(50) NOT NULL,
  block_height BIGINT NOT NULL,
  success BOOLEAN NOT NULL,
  details TEXT,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (name, block_height)
);