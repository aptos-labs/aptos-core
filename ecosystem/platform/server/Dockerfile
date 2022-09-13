# syntax=docker/dockerfile:1.4

# NOTE: We're only using this base image for the preinstalled dependencies. We
# are using Puma as the app server, not Passenger.
FROM phusion/passenger-full:2.3.0@sha256:dfd15f7a63ae07b8637df6a01a603f8ae82cbbdf684bfd696d09ae130c034fd1

RUN npm install -g yarn && \
  rm -f /etc/my_init.d/10_syslog-ng.init && \
  mkdir /home/app/webapp && \
  chown app: /home/app/webapp

RUN apt-get update && apt-get install -y \
  imagemagick libmagickcore-dev \
  pngquant \
  libvips \
  && rm -rf /var/lib/apt/lists/*

USER app
ENV HOME /home/app
ENV RAILS_ENV production
ENV RAILS_LOG_TO_STDOUT true
ENV RAILS_SERVE_STATIC_FILES true
ENV NODE_ENV production
ENV SENTRY_FRONTEND_DSN https://ef8e0deb57234fe2bf276abab441e42a@o1162451.ingest.sentry.io/6369000
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
