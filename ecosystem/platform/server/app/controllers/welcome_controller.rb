# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class WelcomeController < ApplicationController
  def index
    @hide_header = true
    redirect_to overview_index_path if user_signed_in?
  end

  def it1
    store_location_for(:user, it1_path) unless user_signed_in?
  end
end
