-- Your SQL goes here

/** Ex:
{
  "key": "0x0400000000000000000000000000000000000000000000000000000000000000000000000a550c18",
  "sequence_number": "0",
  "type": "0x1::Reconfiguration::NewEpochEvent",
  "data": {
    "epoch": "1"
  }
}
 */

CREATE TABLE events
(
    -- join from "transactions"
    transaction_hash VARCHAR(255) NOT NULL,

    key              VARCHAR(100) NOT NULL,
    sequence_number  BIGINT       NOT NULL,
    type             TEXT         NOT NULL,
    data             jsonb        NOT NULL,

    -- Default time columns
    inserted_at      TIMESTAMP    NOT NULL DEFAULT NOW(),

    -- Constraints
    PRIMARY KEY (key, sequence_number),
    CONSTRAINT fk_transactions
        FOREIGN KEY (transaction_hash)
            REFERENCES transactions (hash)
);

CREATE INDEX event_key_seq_type_index ON events (key, sequence_number, type);
