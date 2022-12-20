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
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);