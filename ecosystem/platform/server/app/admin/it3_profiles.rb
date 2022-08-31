# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register It3Profile do
  menu priority: 2
  actions :all, except: %i[destroy edit new]

  permit_params :consensus_key
  includes :user

  index do
    selectable_column
    id_column
    column :user
    column :owner_key
    column :consensus_key
    column :account_key
    column :network_key
    column :validator_ip
    column :validator_address
    column :validator_port
    column :validator_metrics_port
    column :validator_api_port
    column :validator_verified
    column :fullnode_address
    column :fullnode_port
    column :fullnode_metrics_port
    column :fullnode_api_port
    column :fullnode_network_key
    column :created_at
    column :updated_at
    actions
  end

  filter :owner_key
  filter :consensus_key
  filter :account_key
  filter :network_key
  filter :validator_ip
  filter :validator_address
  filter :validator_port
  filter :validator_metrics_port
  filter :validator_api_port
  filter :fullnode_address
  filter :fullnode_port
  filter :fullnode_metrics_port
  filter :fullnode_api_port
  filter :validator_verified
  filter :created_at
  filter :updated_at

  show do
    default_main_content do
      row :location
      row(:it3_survey) { |profile| profile.user.it3_survey }
    end
  end
end
