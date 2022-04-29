# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class Ability
  include CanCan::Ability

  def initialize(user)
    can :manage, :all if user&.is_root?
  end
end
