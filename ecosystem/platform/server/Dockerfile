# syntax=docker/dockerfile:1.4

# NOTE: We're only using this base image for the preinstalled dependencies. We
# are using Puma as the app server, not Passenger.
FROM phusion/passenger-full:2.3.0

RUN npm install -g yarn && \
  rm -f /etc/my_init.d/10_syslog-ng.init && \
  mkdir /home/app/webapp && \
  chown app: /home/app/webapp

USER app
ENV HOME /home/app
ENV RAILS_ENV production
ENV RAILS_LOG_TO_STDOUT true
ENV RAILS_SERVE_STATIC_FILES true
ENV NODE_ENV production
WORKDIR /home/app/webapp

COPY --chown=app:app Gemfile Gemfile.lock .

RUN bundle config --global frozen 1 && \
  bundle config set --local without 'development test' && \
  bundle install

COPY --chown=app:app package.json yarn.lock .

RUN yarn install

COPY --chown=app:app . /home/app/webapp

RUN yarn build && yarn build:css && \
  SECRET_KEY_BASE=$(bin/rake secret) SKIP_DB_CHECK=1 bundle exec rake assets:precompile

EXPOSE 3000

CMD ["bundle", "exec", "rails", "server", "--binding", "0.0.0.0", "--port", "3000"]
