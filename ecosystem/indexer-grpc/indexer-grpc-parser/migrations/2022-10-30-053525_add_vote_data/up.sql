-- Your SQL goes here
-- Add token_name to join with token tables, {subdomain}.{domain}.apt
ALTER TABLE current_ans_lookup
ADD COLUMN token_name VARCHAR(140) NOT NULL DEFAULT '';
CREATE INDEX ans_tn_index ON current_ans_lookup (token_name);
-- Add voting table
CREATE TABLE proposal_votes (
  transaction_version BIGINT NOT NULL,
  proposal_id BIGINT NOT NULL,
  voter_address VARCHAR(66) NOT NULL,
  staking_pool_address VARCHAR(66) NOT NULL,
  num_votes NUMERIC NOT NULL,
  should_pass BOOLEAN NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, proposal_id, voter_address)
);
CREATE INDEX pv_pi_va_index ON proposal_votes (proposal_id, voter_address);
CREATE INDEX pv_va_index ON proposal_votes (voter_address);
CREATE INDEX pv_spa_index ON proposal_votes (staking_pool_address);
CREATE INDEX pv_ia_index ON proposal_votes (inserted_at);