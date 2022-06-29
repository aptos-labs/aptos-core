# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class SettingsController < ApplicationController
  before_action :authenticate_user!
  before_action :set_user

  layout 'settings'

  def profile; end

  def profile_update
    user_params = params.fetch(:user, {}).permit(:username, :email)
    if @user.update(user_params)
      notice = if @user.pending_reconfirmation?
                 "A verification email has been sent to #{@user.unconfirmed_email}."
               else
                 'User profile updated'
               end
      redirect_to settings_profile_url, notice:
    else
      render :profile, status: :unprocessable_entity
    end
  end

  def connections
    store_location_for(current_user, request.path)
    @authorizations = @user.authorizations.group_by(&:provider)
  end

  def connections_delete
    store_location_for(current_user, request.path)
    @authorizations = @user.authorizations

    auth_id = params.fetch(:authorization, {}).require(:id)
    authorization = @authorizations.select { |auth| auth.id == auth_id }.first

    if authorization.nil?
      flash[:alert] = 'Connection not found.'
      render :connections, status: :unprocessable_entity
    elsif @authorizations.length == 1
      flash[:alert] = 'Cannot remove the last connection.'
      render :connections, status: :unprocessable_entity
    elsif authorization.destroy
      redirect_to settings_connections_url, notice: 'Connection removed.'
    else
      flash[:alert] = 'Unable to remove connection.'
      render :connections, status: :unprocessable_entity
    end
  end

  private

  def set_user
    @user = current_user
  end
end
