# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class SendConfirmEmailJob < SendEmailJob
  # Ex args: { user_id: 32, template_vars: { CONFIRM_LINK: 'https://...' } }

  private

  def template_name
    :confirm
  end
end
