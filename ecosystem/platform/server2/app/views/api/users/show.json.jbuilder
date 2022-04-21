# frozen_string_literal: true

json.extract! @user, :id, :username, :is_developer, :is_node_operator

# Only show certain details for our own profile (or if we're an admin)
if current_user.id == @user.id || current_user.is_root?
  json.extract! @user, :providers, :is_root, :mainnet_address, :kyc_status
end
