# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AddNotifiedAtToNetworkOperation < ActiveRecord::Migration[7.0]
  def change
    add_column :network_operations, :notified_at, :datetime,
               comment: 'The time at which a notification was sent for this network operation.'
  end
end
