-- Your SQL goes here
CREATE TABLE petra_activities (
  transaction_version BIGINT NOT NULL,
  account_address VARCHAR(66) NOT NULL,
  coin_activities jsonb NOT NULL,
  token_activities jsonb NOT NULL,
  PRIMARY KEY (account_address, transaction_version)
);
