-- Your SQL goes here

CREATE TABLE collections
(
    creator VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description VARCHAR,
    maxAmount bigint,
    uri VARCHAR,
    created_at TIMESTAMP NOT NULL,

    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (creator, name)
);
