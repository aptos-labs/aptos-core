-- Your SQL goes here
CREATE TABLE tokens
(
   token_id VARCHAR NOT NULL,
   creator VARCHAR NOT NULL,
   collection VARCHAR NOT NULL,
   name VARCHAR NOT NULL,
   description VARCHAR NOT NULL,
   max_amount bigint,
   supply bigint NOT NULL,
   uri VARCHAR NOT NULL,
   minted_at TIMESTAMP NOT NULL,
   last_minted_at TIMESTAMP NOT NULL,

   inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Constraints
   PRIMARY KEY (token_id)
);
