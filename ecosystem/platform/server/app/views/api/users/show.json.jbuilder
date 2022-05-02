# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

json.extract! @user, :id, :username, :is_developer, :is_node_operator

# Only show certain details for our own profile (or if we're an admin)
if current_user&.id == @user.id || current_user&.is_root?
  json.extract! @user, :is_root, :mainnet_address, :kyc_status, :email, :confirmed_at
  json.authorizations(@user.authorizations) do |authorization|
    json.extract! authorization, :provider, :username, :expires, :expires_at
  end
end

# Used after registration
json._message @message if @message
