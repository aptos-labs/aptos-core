# frozen_string_literal: true

require_relative "lib/pr_ci_policy"

exit(PrCiPolicy.run_from_environment)
