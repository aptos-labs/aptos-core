# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register Location do
  menu priority: 2
  actions :all, except: %i[destroy edit new]

  permit_params :consensus_key

  index do
    selectable_column
    id_column
    column :item
    column :ip_address
    column :latitude
    column :longitude
    column(:isp) { |loc| loc.isp.presence || loc.organization }
    column :continent_name
    column :country_name
    column :subdivision_name
    column :city_name
    column :created_at
    column :updated_at
    actions
  end

  filter :ip_address
  filter :latitude
  filter :longitude
  filter :isp
  filter :continent_name
  filter :country_name
  filter :subdivision_name
  filter :city_name
  filter :created_at
  filter :updated_at
end
