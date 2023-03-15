-- Your SQL goes here
CREATE TABLE nft_points (
  transaction_version BIGINT UNIQUE PRIMARY KEY NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  token_name TEXT NOT NULL,
  point_type TEXT NOT NULL,
  amount NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX np_oa_idx ON nft_points (owner_address);
CREATE INDEX np_tt_oa_idx ON nft_points (transaction_timestamp, owner_address);
CREATE INDEX np_insat_idx ON nft_points (inserted_at);
