# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# These tables are excluded from db/schema.rb.
ActiveRecord::SchemaDumper.ignore_tables = [/^directus_/]
