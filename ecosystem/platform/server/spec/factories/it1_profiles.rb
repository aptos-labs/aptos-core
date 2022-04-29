# frozen_string_literal: true

FactoryBot.define do
  factory :it1_profile do
    user { nil }
    consensus_key { 'MyString' }
    account_key { 'MyString' }
    network_key { 'MyString' }
    validator_address { 'MyString' }
    validator_port { 1 }
    metrics_port { 1 }
    fullnode_address { 'MyString' }
    fullnode_port { 1 }
  end
end
