# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UsersController < ApplicationController
  before_action :ensure_profiles_enabled!
  layout 'users'

  # GET /users/1
  def show
    @user = User.find(params[:id])
  end

  # GET /users/1/projects
  def projects
    @user = User.find(params[:user_id])
    @projects = Project
                .joins(:project_members)
                .where(project_members: { user_id: @user.id, public: true })
                .or(Project.where(user: @user))
                .where(public: true)
                .distinct
                .with_attached_thumbnail
  end

  # GET /users/1/activity
  def activity
    @user = User.find(params[:user_id])
  end

  # GET /users/1/rewards
  def rewards
    @user = User.find(params[:user_id])
  end

  private

  def ensure_profiles_enabled!
    redirect_to root_path unless Flipper.enabled?(:profiles)
  end
end
