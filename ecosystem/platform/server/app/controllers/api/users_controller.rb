# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Api
  class UsersController < ::ApiController
    before_action :authenticate_user!, only: %i[update me]

    def me
      @user = current_user
      render 'api/users/show', formats: [:json]
    end

    def show
      @user = User.includes(:authorizations).find(params[:id])
      render 'api/users/show', formats: [:json]
    end

    def update
      user_params = params.permit(:username, :is_developer, :is_node_operator, :mainnet_address)
      @user = User.includes(:authorizations).find(params[:id])
      # Only let users change their own profile
      head :forbidden unless @user.id == current_user.id
      @user.update!(user_params)
      render 'api/users/show', formats: [:json]
    end
  end
end
