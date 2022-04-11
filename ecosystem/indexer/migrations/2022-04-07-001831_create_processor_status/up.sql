-- Your SQL goes here


CREATE TABLE processor_statuses
(
    name         VARCHAR(50) NOT NULL,
    version      BIGINT      NOT NULL,
    success      BOOLEAN     NOT NULL,
    details      TEXT,
    last_updated TIMESTAMP   NOT NULL DEFAULT NOW(),

    -- Constraints
    PRIMARY KEY (name, version)
);
