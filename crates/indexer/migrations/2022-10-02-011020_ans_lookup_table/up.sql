-- Your SQL goes here
-- add indices for current ownership to speed up queries
CREATE INDEX curr_to_oa_tt_am_ltv_index ON current_token_ownerships (
  owner_address,
  table_type,
  amount,
  last_transaction_version DESC
);
CREATE INDEX curr_to_oa_tt_ltv_index ON current_token_ownerships (
  owner_address,
  table_type,
  last_transaction_version DESC
);
-- allows quick lookup for velor name services registered address
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