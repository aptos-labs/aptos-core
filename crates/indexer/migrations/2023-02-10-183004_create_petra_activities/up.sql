-- Your SQL goes here
CREATE TABLE petra_activities (
  transaction_version BIGINT NOT NULL,
  account_address VARCHAR(66) NOT NULL,
  PRIMARY KEY (account_address, transaction_version)
);
