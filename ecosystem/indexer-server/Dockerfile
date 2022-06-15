FROM node:16 as builder
ARG PATH

WORKDIR /usr

RUN echo "copy indexer server..."
COPY ./doc ./doc
COPY ./typescript/prisma ./typescript/prisma
COPY ./typescript/src ./typescript/src
COPY ./typescript/package.json ./typescript/package.json
COPY ./typescript/tsconfig.json ./typescript/tsconfig.json

WORKDIR /usr/typescript

RUN echo "install and generate the server..."
RUN yarn install
RUN yarn run generate

EXPOSE 4000
CMD ["yarn", "run", "dev"]
