# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UserTest < ActiveSupport::TestCase
  include Devise::Test::IntegrationHelpers

  test 'username regex' do
    # can't begin/end with a special
    refute_match User::USERNAME_REGEX, 'bad_'
    refute_match User::USERNAME_REGEX, '_bad'
    refute_match User::USERNAME_REGEX, '_bad_'
    refute_match User::USERNAME_REGEX, '__bad__'
    refute_match User::USERNAME_REGEX, 'bad-'
    refute_match User::USERNAME_REGEX, '-bad'
    refute_match User::USERNAME_REGEX, '-bad-'
    refute_match User::USERNAME_REGEX, '-bad--'

    # can't have two specials in a row
    refute_match User::USERNAME_REGEX, 'bad-_bad'
    refute_match User::USERNAME_REGEX, 'bad-_bad'
    refute_match User::USERNAME_REGEX, 'bad--bad'
    refute_match User::USERNAME_REGEX, 'bad__bad'

    # or invalid characters
    refute_match User::USERNAME_REGEX, 'no good'

    # These are all valid
    assert_match User::USERNAME_REGEX, 'potatosalad'
    assert_match User::USERNAME_REGEX, 'potato-5-salad'
    assert_match User::USERNAME_REGEX, 'potato-5salad'
    assert_match User::USERNAME_REGEX, 'potato_5_salad'
    assert_match User::USERNAME_REGEX, 'p-o_t-a_t_o-5-s_a_l-a_d'
    assert_match User::USERNAME_REGEX, '555yes111'

    assert_equal(User::USERNAME_REGEX_JS,
                 '^(?!^[\\-_])(?!.*[\\-_]{2,})(?!.*[\\-_]$)[a-zA-Z0-9\\-_]+$')
  end
end
