# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Delayed.logger = Rails.logger

Delayed.default_log_level = 'debug'

Delayed::Worker.max_run_time = 20.minutes

Delayed::Worker.read_ahead = 1
