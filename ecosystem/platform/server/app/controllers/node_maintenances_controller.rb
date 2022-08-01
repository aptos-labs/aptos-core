# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NodeMaintenancesController < ApplicationController
  def index
    @node_maintenances = NodeMaintenance.order(created_at: :desc)
  end

  def show
    @node_maintenance = NodeMaintenance.find(params[:id])
  end
end
