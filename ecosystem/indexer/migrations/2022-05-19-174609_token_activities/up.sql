-- Your SQL goes here
CREATE TABLE token_activities
(
    event_key VARCHAR NOT NULL,
    sequence_number BIGINT NOT NULL,
    account VARCHAR NOT NULL,
    token_id VARCHAR,
    event_type VARCHAR,
    amount NUMERIC,
    created_at TIMESTAMP NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    transaction_hash VARCHAR(255) NOT NULL,
    PRIMARY KEY (event_key, sequence_number),
    CONSTRAINT fk_transactions
        FOREIGN KEY (transaction_hash)
            REFERENCES transactions (hash)
);
