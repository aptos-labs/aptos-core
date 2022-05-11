# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class OverviewController < ApplicationController
  before_action :authenticate_user!

  def index; end
end
