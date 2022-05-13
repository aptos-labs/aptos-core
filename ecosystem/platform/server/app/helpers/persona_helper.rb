# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'httparty'

class PersonaURL
  # @param [URI] parsed_url
  def initialize(parsed_url)
    @parsed = parsed_url
    query = parsed_url.query
    @query = if query.present?
               CGI.parse(query)
             else
               {}
             end
  end

  # @param [String] key
  # @param [Object] value
  # @return [PersonaURL]
  def set_param(key, value)
    @query[key] = value
    self
  end

  def to_s
    @parsed.query = URI.encode_www_form(@query)
    @parsed.to_s
  end
end

FirstInquiryResult = Struct.new(:exists?, :inquiry_id, :status)

module PersonaHelper
  class PersonaClient
    include HTTParty
    base_uri 'https://withpersona.com/api/v1/'
    headers({ 'Authorization' => "Bearer #{ENV.fetch('PERSONA_API_KEY', 'test')}", 'Persona-Version' => '2021-05-14',
              'Key-Inflection' => 'snake_case' })

    # TODO: support idempotency key (user.external_id is fine)

    # @param [String] reference_id
    # @return [FirstInquiryResult, nil]
    def inquiry_id_from_reference_id(reference_id)
      inquiries = get_inquiries(reference_id:)
      first_inquiry = inquiries['data']&.first
      if first_inquiry.present?
        FirstInquiryResult.new(true, first_inquiry['id'], first_inquiry['attributes']['status'])
      else
        FirstInquiryResult.new(false, nil, nil)
      end
    end

    # https://docs.withpersona.com/reference/list-all-inquiries
    # @param [Integer] after
    # @param [String] account_id
    # @param [String] reference_id
    def get_inquiries(after: nil, account_id: nil, reference_id: nil)
      query = {
        'page[after]' => after,
        'filter[account-id]' => account_id,
        'filter[reference-id]' => reference_id,
        'page[size]' => 50
      }
      self.class.get('/inquiries', { query: })
    end

    # https://docs.withpersona.com/reference/apiv1inquiriesinquiry-id
    def inquiry(inquiry_id)
      self.class.get("/inquiries/#{inquiry_id}")
    end
  end

  class PersonaInvite
    # @param [User] user
    def initialize(user)
      @user = user
    end

    def url
      PersonaURL.new(URI.parse(base_url))
                .set_param('reference-id', @user.external_id)
    end

    private

    def base_url
      ENV.fetch('PERSONA_URL_PREFIX', ' https://withpersona.com/verify?inquiry-template-id=itmpl_X1pKpnefCS8wShXJBtXyfRSf&environment=sandbox')
    end
  end
end
