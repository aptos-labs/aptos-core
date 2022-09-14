# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddVerifiedToProjects < ActiveRecord::Migration[7.0]
  def change
    add_column :projects, :verified, :boolean, null: false, default: false
  end
end
