CREATE TABLE marketplace_collections (
    creator_address VARCHAR(66) NOT NULL,
    collection_name TEXT NOT NULL,

    PRIMARY KEY (
        creator_address,
        collection_name
    ),

    CONSTRAINT collection_id UNIQUE(creator_address, collection_name)
)

CREATE TABLE marketplace_offers (
    creator_address VARCHAR(66) NOT NULL,
    collection_name TEXT NOT NULL,
    token_name TEXT NOT NULL,
    property_version SMALLINT NOT NULL,
    price BIGINT NOT NULL,
    seller VARCHAR(66) NOT NULL
    "timestamp" TIMESTAMP NOT NULL

    PRIMARY KEY (
        creator_address,
        collection_name
    ),

    CONSTRAINT collection_id UNIQUE(creator_address, collection_name)

    CONSTRAINT FK_creator_address 
    FOREIGN KEY (creator_address)
    REFERENCES creator_address (marketplace_collections)

    CONSTRAINT FK_collection_name 
    FOREIGN KEY (collection_name)
    REFERENCES collection_name (marketplace_collections)
)

CREATE TABLE marketplace_orders (
    creator_address VARCHAR(66) NOT NULL,
    collection_name TEXT NOT NULL,
    token_name TEXT NOT NULL,
    property_version SMALLINT NOT NULL,
    price BIGINT NOT NULL,
    quantity BIGINT NOT NULL,
    maker VARCHAR(66) NOT NULL
    "timestamp" TIMESTAMP NOT NULL

    PRIMARY KEY (
        creator_address,
        collection_name
    ),

    CONSTRAINT collection_id UNIQUE(creator_address, collection_name)

    CONSTRAINT FK_creator_address 
    FOREIGN KEY (creator_address)
    REFERENCES creator_address (marketplace_collections)

    CONSTRAINT FK_collection_name 
    FOREIGN KEY (collection_name)
    REFERENCES collection_name (marketplace_collections)
)

CREATE TABLE marketplace_bids (
    creator_address VARCHAR(66) NOT NULL,
    collection_name TEXT NOT NULL,
    token_name TEXT NOT NULL,
    property_version SMALLINT NOT NULL,
    price BIGINT NOT NULL,
    maker VARCHAR(66) NOT NULL
    "timestamp" TIMESTAMP NOT NULL

    PRIMARY KEY (
        creator_address,
        collection_name
    ),

    CONSTRAINT collection_id UNIQUE(creator_address, collection_name)

    CONSTRAINT FK_creator_address 
    FOREIGN KEY (creator_address)
    REFERENCES creator_address (marketplace_collections)

    CONSTRAINT FK_collection_name 
    FOREIGN KEY (collection_name)
    REFERENCES collection_name (marketplace_collections)
)