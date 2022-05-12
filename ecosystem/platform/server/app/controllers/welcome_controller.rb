# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WelcomeController < ApplicationController
  layout 'it1'

  before_action :ensure_confirmed!, only: %i[it1]

  def index; end

  def it1
    redirect_to root_path unless user_signed_in?
  end
end
