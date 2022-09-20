/* Genesis Tx (doesn't have an entry in user_transactions or block_metadata_transactions) Ex:
 {
 "type":"genesis_transaction",
 "version":"0",
 "hash":"0x12180a4bbccf48de4d1e23b498add134328669ffc7741c8d529c6b2e3629ac99",
 "state_root_hash":"0xb50adef3662d77e528be9e1cb5637fe5b7afd13eea317b330799f0c559c918c1",
 "event_root_hash":"0xcbdbb1b830d1016d45a828bb3171ea81826e8315f14140acfbd7886f49fbcb40",
 "gas_used":"0",
 "success":true,
 "vm_status":"Executed successfully",
 "accumulator_root_hash":"0x188ed588547d551e652f04fccd5434c2977d6cff9e7443eb8e7c3038408caad4",
 "payload":{
 "type":"write_set_payload",
 "write_set":{
 "type":"direct_write_set",
 "changes":[],
 "events":[]
 }
 },
 "events":[
 {
 "key":"0x0400000000000000000000000000000000000000000000000000000000000000000000000a550c18",
 "sequence_number":"0",
 "type":"0x1::reconfiguration::NewEpochEvent",
 "data":{
 "epoch":"1"
 }
 }
 ]
 }
 */
CREATE TABLE transactions (
  version BIGINT UNIQUE PRIMARY KEY NOT NULL,
  block_height BIGINT NOT NULL,
  hash VARCHAR(66) UNIQUE NOT NULL,
  type VARCHAR(50) NOT NULL,
  payload jsonb,
  state_change_hash VARCHAR(66) NOT NULL,
  event_root_hash VARCHAR(66) NOT NULL,
  state_checkpoint_hash VARCHAR(66),
  gas_used NUMERIC NOT NULL,
  success BOOLEAN NOT NULL,
  vm_status TEXT NOT NULL,
  accumulator_root_hash VARCHAR(66) NOT NULL,
  num_events BIGINT NOT NULL,
  num_write_set_changes BIGINT NOT NULL,
  -- Default time columns
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX txn_insat_index ON transactions (inserted_at);
/* Ex:
 {
 "type":"block_metadata_transaction",
 "version":"69158",
 "hash":"0x2b7c58ed8524d228f9d0543a82e2793d04e8871df322f976b0e7bb8c5ced4ff5",
 "state_root_hash":"0x3ead9eb40582fbc7df5e02f72280931dc3e6f1aae45dc832966b4cd972dac4b8",
 "event_root_hash":"0x2e481956dea9c59b6fc9f823fe5f4c45efce173e42c551c1fe073b5d76a65504",
 "gas_used":"0",
 "success":true,
 "vm_status":"Executed successfully",
 "accumulator_root_hash":"0xb0ad602f805eb20c398f0f29a3504a9ef38bcc52c9c451deb9ec4a2d18807b49",
 "id":"0xeef99391a3fc681f16963a6c03415bc0b1b12b56c00429308fa8bf46ac9eddf0",
 "round":"57600",
 "previous_block_votes":[
 "0x992da26d46e6d515a070c7f6e52376a1e674e850cb4d116babc6f870da9c258",
 "0xfb4d785594a018bd980b4a20556d120c53a3f50b1cff9d5aa2e26eee582a587",
 "0x2b7bce01a6f55e4a863c4822b154021a25588250c762ee01169b6208d6169208",
 "0x43a2c4cefc4725e710dadf423dd9142057208e640c623b27c6bba704380825ab",
 "0x4c91f3949924e988144550ece1da1bd9335cbecdd1c3ce1893f80e55376d018f",
 "0x61616c1208b6b3491496370e7783d48426c674bdd7d04ed1a96afe2e4d8a3930",
 "0x66ccccae2058641f136b79792d4d884419437826342ba84dfbbf3e52d8b3fc7d",
 "0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
 "0x6bbf2564ea4a6968df450da786b40b3f56b533a7b700c681c31b3714fc30256b",
 "0x735c0a1cb33689ecba65907ba05a485f98831ff610955a44abf0a986f2904612",
 "0x784a9514644c8ab6235aaff425381f2ea2719315a51388bc1f1e1c5afa2daaa9",
 "0x7a8cee78757dfe0cee3631208cc81f171d27ca6004c63ebae5814e1754a03c79",
 "0x803160c3a2f8e025df5a6e1110163493293dc974cc8abd43d4c1896000f4a1ec",
 "0xcece26ebddbadfcfbc541baddc989fa73b919b82915164bbf77ebd86c7edbc90",
 "0xe7be8996cbdf7db0f64abd17aa0968074b32e4b0df6560328921470e09fd608b"
 ],
 "proposer":"0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
 "timestamp":"1649395495746947"
 }
 */
CREATE TABLE block_metadata_transactions (
  version BIGINT UNIQUE PRIMARY KEY NOT NULL,
  block_height BIGINT UNIQUE NOT NULL,
  id VARCHAR(66) NOT NULL,
  round BIGINT NOT NULL,
  epoch BIGINT NOT NULL,
  previous_block_votes_bitvec jsonb NOT NULL,
  proposer VARCHAR(66) NOT NULL,
  failed_proposer_indices jsonb NOT NULL,
  "timestamp" TIMESTAMP NOT NULL,
  -- Default time columns
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  CONSTRAINT fk_versions FOREIGN KEY (version) REFERENCES transactions (version)
);
CREATE INDEX bmt_insat_index ON block_metadata_transactions (inserted_at);
/* Ex:
 {
 "type":"user_transaction",
 "version":"691595",
 "hash":"0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb",
 "state_root_hash":"0xebfe1eb7aa5321e7a7d741d927487163c34c821eaab60646ae0efd02b286c97c",
 "event_root_hash":"0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
 "gas_used":"43",
 "success":true,
 "vm_status":"Executed successfully",
 "accumulator_root_hash":"0x97bfd5949d32f6c9a9efad93411924bfda658a8829de384d531ee73c2f740971",
 "sender":"0xdfd557c68c6c12b8c65908b3d3c7b95d34bb12ae6eae5a43ee30aa67a4c12494",
 "sequence_number":"21386",
 "max_gas_amount":"1000",
 "gas_unit_price":"1",
 "expiration_timestamp_secs":"1649713172",
 "payload":{
 "type":"entry_function_payload",
 "function":"0x1::aptos_coin::mint",
 "type_arguments":[
 
 ],
 "arguments":[
 "0x45b44793724a5ecc6ad85fa60949d0824cfc7f61d6bd74490b13598379313142",
 "20000"
 ]
 },
 "signature":{
 "type":"ed25519_signature",
 "public_key":"0x14ff6646855dad4a2dab30db773cdd4b22d6f9e6813f3e50142adf4f3efcf9f8",
 "signature":"0x70781112e78cc8b54b86805c016cef2478bccdef21b721542af0323276ab906c989172adffed5bf2f475f2ec3a5b284a0ac46a6aef0d79f0dbb6b85bfca0080a"
 },
 "events":[
 {
 "key":"0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
 "sequence_number":"0",
 "type":"0x1::Whatever::FakeEvent1",
 "data":{
 "amazing":"1"
 }
 },
 {
 "key":"0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
 "sequence_number":"1",
 "type":"0x1::Whatever::FakeEvent2",
 "data":{
 "amazing":"2"
 }
 }
 ],
 "timestamp":"1649713141723410"
 }
 */
CREATE TABLE user_transactions (
  version BIGINT UNIQUE PRIMARY KEY NOT NULL,
  block_height BIGINT NOT NULL,
  parent_signature_type VARCHAR(50) NOT NULL,
  sender VARCHAR(66) NOT NULL,
  sequence_number BIGINT NOT NULL,
  max_gas_amount NUMERIC NOT NULL,
  expiration_timestamp_secs TIMESTAMP NOT NULL,
  gas_unit_price NUMERIC NOT NULL,
  -- from UserTransaction
  "timestamp" TIMESTAMP NOT NULL,
  entry_function_id_str text NOT NULL,
  -- Default time columns
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  CONSTRAINT fk_versions FOREIGN KEY (version) REFERENCES transactions (version),
  UNIQUE (sender, sequence_number)
);
CREATE INDEX ut_sender_seq_index ON user_transactions (sender, sequence_number);
CREATE INDEX ut_insat_index ON user_transactions (inserted_at);
-- tracks signatures for user transactions
CREATE TABLE signatures (
  transaction_version BIGINT NOT NULL,
  multi_agent_index BIGINT NOT NULL,
  multi_sig_index BIGINT NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  signer VARCHAR(66) NOT NULL,
  is_sender_primary BOOLEAN NOT NULL,
  type VARCHAR(50) NOT NULL,
  public_key VARCHAR(66) NOT NULL,
  signature VARCHAR(200) NOT NULL,
  threshold BIGINT NOT NULL,
  public_key_indices jsonb NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    multi_agent_index,
    multi_sig_index,
    is_sender_primary
  ),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX sig_insat_index ON signatures (inserted_at);
/** Ex:
 {
 "key": "0x0400000000000000000000000000000000000000000000000000000000000000000000000a550c18",
 "sequence_number": "0",
 "type": "0x1::reconfiguration::NewEpochEvent",
 "data": {
 "epoch": "1"
 }
 }
 */
CREATE TABLE events (
  sequence_number BIGINT NOT NULL,
  creation_number BIGINT NOT NULL,
  account_address VARCHAR(66) NOT NULL,
  transaction_version BIGINT NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  type TEXT NOT NULL,
  data jsonb NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    account_address,
    creation_number,
    sequence_number
  ),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX ev_addr_type_index ON events (account_address);
CREATE INDEX ev_insat_index ON events (inserted_at);
-- write set changes
CREATE TABLE write_set_changes (
  transaction_version BIGINT NOT NULL,
  index BIGINT NOT NULL,
  hash VARCHAR(66) NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  type TEXT NOT NULL,
  address VARCHAR(66) NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, index),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX wsc_addr_type_ver_index ON write_set_changes (address, transaction_version DESC);
CREATE INDEX wsc_insat_index ON write_set_changes (inserted_at);
-- move modules in write set changes
CREATE TABLE move_modules (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  name TEXT NOT NULL,
  address VARCHAR(66) NOT NULL,
  bytecode bytea,
  friends jsonb,
  exposed_functions jsonb,
  structs jsonb,
  is_deleted BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, write_set_change_index),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX mm_addr_name_ver_index ON move_modules (address, name, transaction_version);
CREATE INDEX mm_insat_index ON move_modules (inserted_at);
-- move resources in write set changes
CREATE TABLE move_resources (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  name TEXT NOT NULL,
  address VARCHAR(66) NOT NULL,
  type TEXT NOT NULL,
  module TEXT NOT NULL,
  generic_type_params jsonb,
  data jsonb,
  is_deleted BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, write_set_change_index),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX mr_addr_mod_name_ver_index ON move_resources (address, module, name, transaction_version);
CREATE INDEX mr_insat_index ON move_resources (inserted_at);
-- table items in write set changes
CREATE TABLE table_items (
  key text NOT NULL,
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  transaction_block_height BIGINT NOT NULL,
  table_handle VARCHAR(66) NOT NULL,
  decoded_key jsonb NOT NULL,
  decoded_value jsonb,
  is_deleted BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, write_set_change_index),
  CONSTRAINT fk_transaction_versions FOREIGN KEY (transaction_version) REFERENCES transactions (version)
);
CREATE INDEX ti_hand_ver_key_index ON table_items (table_handle, transaction_version);
CREATE INDEX ti_insat_index ON table_items (inserted_at);
-- table metadatas from table items
CREATE TABLE table_metadatas (
  handle VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  key_type text NOT NULL,
  value_type text NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX tm_insat_index ON table_metadatas (inserted_at);
-- table metadatas in write set changes
CREATE TABLE processor_statuses (
  name VARCHAR(50) NOT NULL,
  version BIGINT NOT NULL,
  success BOOLEAN NOT NULL,
  details TEXT,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (name, version)
);
CREATE INDEX ps_succ_ver_index ON processor_statuses (success, version ASC);
CREATE INDEX ps_ver_index ON processor_statuses (version ASC);
CREATE INDEX ps_lastup_index ON processor_statuses (last_updated);
CREATE TABLE ledger_infos (chain_id BIGINT UNIQUE PRIMARY KEY NOT NULL);