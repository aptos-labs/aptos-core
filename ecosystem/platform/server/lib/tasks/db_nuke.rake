# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

namespace :db do
  desc 'Drop all tables'
  task nuke: :environment do
    conn = ActiveRecord::Base.connection
    query = "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname='public'"
    tables = conn.execute(query).map { |r| r['tablename'] }
    tables.each { |t| conn.execute("DROP TABLE \"#{t}\" CASCADE") }
  end
end
