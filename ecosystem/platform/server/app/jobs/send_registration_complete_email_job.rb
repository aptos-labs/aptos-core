# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class SendRegistrationCompleteEmailJob < SendEmailJob
  # Ex args: { user_id: 32 }

  private

  def template_name
    :'registration-complete'
  end
end
