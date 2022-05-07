# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class WelcomeController < ApplicationController
  before_action :authenticate_user!, only: %i[it1]

  def index
    redirect_to overview_index_path if user_signed_in?
  end

  def it1; end
end
