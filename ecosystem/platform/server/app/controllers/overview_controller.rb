# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class OverviewController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_confirmed!

  def index
    redirect_to root_path
  end
end
