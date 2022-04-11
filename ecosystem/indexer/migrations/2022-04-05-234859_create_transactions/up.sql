-- Your SQL goes here

CREATE TABLE transactions
(
    type                  VARCHAR(255)                    NOT NULL,
    payload               jsonb                           NOT NULL,

    -- from OnChainTransactionInfo
    version               BIGINT UNIQUE                   NOT NULL,
    hash                  VARCHAR(255) UNIQUE PRIMARY KEY NOT NULL,
    state_root_hash       VARCHAR(255)                    NOT NULL,
    event_root_hash       VARCHAR(255)                    NOT NULL,
    gas_used              BIGINT                          NOT NULL,
    success               BOOLEAN                         NOT NULL,
    vm_status             TEXT                            NOT NULL,
    accumulator_root_hash VARCHAR(255)                    NOT NULL,

    -- Default time columns
    inserted_at           TIMESTAMP                       NOT NULL DEFAULT NOW()
);


CREATE TABLE user_transactions
(
    -- join from "transactions"
    hash                      VARCHAR(255) UNIQUE PRIMARY KEY NOT NULL,

    -- from UserTransactionSignature
    signature                 jsonb                           NOT NULL,

    -- from UserTransactionRequest
    sender                    VARCHAR(255)                    NOT NULL,
    sequence_number           BIGINT                          NOT NULL,
    max_gas_amount            BIGINT                          NOT NULL,
    -- ignore 'gas_currency_code', as we'll remove it
    expiration_timestamp_secs TIMESTAMP                       NOT NULL,
    gas_unit_price            BIGINT                          NOT NULL,

    -- from UserTransaction
    "timestamp"               TIMESTAMP                       NOT NULL,

    -- Default time columns
    inserted_at               TIMESTAMP                       NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT fk_transactions
        FOREIGN KEY (hash)
            REFERENCES transactions (hash),
    UNIQUE (sender, sequence_number)
);

CREATE INDEX ut_sender_index ON user_transactions (sender);

CREATE TABLE block_metadata_transactions
(
    -- join from "transactions"
    hash                 VARCHAR(255) UNIQUE PRIMARY KEY NOT NULL,

    -- from BlockMetadataTransaction
    id                   VARCHAR(255)                    NOT NULL,
    round                BIGINT                          NOT NULL,
    previous_block_votes jsonb                           NOT NULL,
    proposer             VARCHAR(255)                    NOT NULL,
    "timestamp"          TIMESTAMP                       NOT NULL,

    -- Default time columns
    inserted_at          TIMESTAMP                       NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT fk_transactions
        FOREIGN KEY (hash)
            REFERENCES transactions (hash)
);
