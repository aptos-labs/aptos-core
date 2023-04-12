-- Your SQL goes here

-- vaults
CREATE TABLE vaults (
  -- Hash of the non-truncated vault type
  type_hash VARCHAR(64) NOT NULL,
  -- module_address::vault::Vault<collateral_type, borrow_type>
  collateral_type VARCHAR(512) NOT NULL,
  borrow_type VARCHAR(512) NOT NULL,
  total_collateral NUMERIC NOT NULL,
  borrow_elastic NUMERIC NOT NULL,
  borrow_base NUMERIC NOT NULL,
  last_fees_accrue_time NUMERIC NOT NULL,
  fees_accrued NUMERIC NOT NULL,
  interest_per_second NUMERIC NOT NULL,
  collateralization_rate NUMERIC NOT NULL,
  liquidation_multiplier NUMERIC NOT NULL,
  borrow_fee NUMERIC NOT NULL,
  distribution_part NUMERIC NOT NULL,
  fee_to VARCHAR(66) NOT NULL,
  cached_exchange_rate NUMERIC NOT NULL,
  last_interest_update NUMERIC NOT NULL,
  is_emergency BOOLEAN NOT NULL,
  dev_cut NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    type_hash
  )
);
CREATE INDEX vault_tv_ct_bt on vaults (transaction_version, collateral_type, borrow_type);

-- user infos
CREATE TABLE user_infos (
  user_address VARCHAR(66) NOT NULL,
  -- Hash of the non-truncated vault type
  type_hash VARCHAR(64) NOT NULL,
  -- module_address::vault::UserInfo<collateral_type, borrow_type>
  collateral_type VARCHAR(512) NOT NULL,
  borrow_type VARCHAR(512) NOT NULL,
  user_collateral NUMERIC NOT NULL,
  user_borrow_part NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    user_address,
    type_hash
  )
);
CREATE INDEX ui_tv_ua_ct_bt on user_infos (transaction_version, user_address, collateral_type, borrow_type);

-- vault activities
CREATE TABLE vault_activities (
  transaction_version BIGINT NOT NULL,
  event_creation_number BIGINT NOT NULL,
  event_sequence_number BIGINT NOT NULL,
  event_index BIGINT NOT NULL,
  -- Hash of the non-truncated vault type
  type_hash VARCHAR(64) NOT NULL,
   -- module_address::vault_events::event_type<collateral_type, borrow_type>
  event_type VARCHAR(5000) NOT NULL,
  collateral_type VARCHAR(5000) NOT NULL,
  borrow_type VARCHAR(5000) NOT NULL,
  collateral_amount NUMERIC,
  borrow_amount NUMERIC,
  user_addr VARCHAR(66),
  withdraw_addr VARCHAR(66),
  liquidator_addr VARCHAR(66),
  accrued_amount NUMERIC,
  rate NUMERIC,
  fees_earned NUMERIC,
  old_interest_per_second NUMERIC,
  new_interest_per_second NUMERIC,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    event_creation_number,
    event_sequence_number
  )
);
CREATE INDEX va_ct_bt_et_sn on vault_activities (collateral_type, borrow_type, event_type, event_sequence_number);
CREATE INDEX va_th_et_sn on vault_activities (type_hash, event_type, event_sequence_number);

