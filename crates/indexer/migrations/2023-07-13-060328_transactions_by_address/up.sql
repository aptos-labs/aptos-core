-- Your SQL goes here
-- Records transactions - account pairs. Account here can represent 
-- user account, resource account, or object account.
CREATE TABLE IF NOT EXISTS account_transactions (
  transaction_version BIGINT NOT NULL,
  account_address VARCHAR(66) NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (account_address, transaction_version)
);
CREATE INDEX IF NOT EXISTS at_version_index ON account_transactions (transaction_version DESC);
CREATE INDEX IF NOT EXISTS at_insat_index ON account_transactions (inserted_at);
ALTER TABLE objects
ALTER COLUMN owner_address
SET NOT NULL;
ALTER TABLE objects
ALTER COLUMN guid_creation_num
SET NOT NULL;
ALTER TABLE objects
ALTER COLUMN allow_ungated_transfer
SET NOT NULL;