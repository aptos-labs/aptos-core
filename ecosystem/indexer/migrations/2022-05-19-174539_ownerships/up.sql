-- Your SQL goes here
CREATE TABLE ownerships
(
    token_id VARCHAR,
    owner VARCHAR,
    amount BIGINT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    PRIMARY KEY (token_id, owner)
);
