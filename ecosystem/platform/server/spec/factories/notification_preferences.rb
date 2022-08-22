# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

FactoryBot.define do
  factory :notification_preference do
    user { nil }
    delivery_method { 0 }
    node_upgrade_notification { false }
    governance_proposal_notification { false }
  end
end
