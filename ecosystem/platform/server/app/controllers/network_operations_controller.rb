# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NetworkOperationsController < ApplicationController
  def index
    @network_operations = NetworkOperation.order(created_at: :desc)
  end

  def show
    @network_operation = NetworkOperation.find(params[:id])
  end
end
