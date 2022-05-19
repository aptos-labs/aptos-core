-- Your SQL goes here

CREATE TABLE collections
(
    creator VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    max_amount bigint,
    uri VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL,

    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (creator, name)
);
