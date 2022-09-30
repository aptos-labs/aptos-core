-- Your SQL goes here
CREATE TABLE current_ans_lookup (
  domain VARCHAR(64) NOT NULL,
  -- if subdomain is null set to empty string
  subdomain VARCHAR(64) NOT NULL,
  registered_address VARCHAR(66),
  expiration_timestamp TIMESTAMP NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (domain, subdomain)
);
CREATE INDEX ans_et_index ON current_ans_lookup (expiration_timestamp);
CREATE INDEX ans_ra_et_index ON current_ans_lookup (registered_address, expiration_timestamp);
CREATE INDEX ans_d_s_et_index ON current_ans_lookup (domain, subdomain, expiration_timestamp);
CREATE INDEX ans_insat_index ON current_ans_lookup (inserted_at);