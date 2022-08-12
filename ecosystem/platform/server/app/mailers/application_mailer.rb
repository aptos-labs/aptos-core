# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApplicationMailer < ActionMailer::Base
  default from: 'community@aptoslabs.com'
  layout 'mailer'
end
