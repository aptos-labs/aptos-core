#!/bin/bash

# SPDX-FileCopyrightText: 2020 Dennis Jenkins
#
# SPDX-License-Identifier: Apache-2.0

# Run this script to create new SSL certs for pistache HTTPS unit tests.
# Passphrases are disabled on all keys.  Do not use these certs/keys in
# production; they are for unit tests only!

# Requires openssl-1.1.1 or greater (for 'extensions' support).

DAYS=3650
BITS=2048

trap "echo 'aborting'; exit 255" 2 3
log() {
  echo -e "\x1b[32m$1\x1b[0m"
}

log "Create rootCA.key and rootCA.crt"
openssl req -x509 -newkey rsa:${BITS} -sha256 -days ${DAYS} -nodes \
  -keyout rootCA.key -out rootCA.crt -subj "/CN=pistache.io" \
  -addext "subjectAltName=IP:127.0.0.1" || exit $?

log "Create server.key"
openssl genrsa -out server.key ${BITS} || exit $?

log "Create server.csr"
openssl req -new -sha256 -key server.key \
  -subj "/C=US/ST=WA/O=Pistache/CN=server" \
  -out server.csr || exit $?

log "Create server.crt"
openssl x509 -req -in server.csr -days ${DAYS} -sha256 -extfile openssl.conf \
  -CA rootCA.crt -CAkey rootCA.key -set_serial 01 \
  -extensions server -out server.crt || exit $?

rm -f server.csr || exit $?

log "Create client.key"
openssl genrsa -out client.key ${BITS} || exit $?

log "Create client.csr"
openssl req -new -sha256 -key client.key \
  -subj "/C=US/ST=WA/O=Pistache/CN=client" \
  -out client.csr || exit $?

log "Create client.crt"
openssl x509 -req -in client.csr -days ${DAYS} -sha256 -extfile openssl.conf \
  -CA rootCA.crt -CAkey rootCA.key -set_serial 02 \
  -extensions client -out client.crt || exit $?

rm -f client.csr || exit $?

log "Create intermediateCA.key"
openssl genrsa -out intermediateCA.key ${BITS} || exit $?

log "Create intermediateCA.csr"
openssl req -new -sha256 -key intermediateCA.key \
  -subj "/C=US/ST=WA/O=Pistache/CN=intermediateCA" \
  -out intermediateCA.csr || exit $?

log "Create intermediateCA.crt"
openssl x509 -req -in intermediateCA.csr -days ${DAYS} -sha256 -extfile openssl.conf \
  -CA rootCA.crt -CAkey rootCA.key -set_serial 01 \
  -extensions intermediateCA -out intermediateCA.crt || exit $?

rm -f intermediateCA.csr || exit $?

log "Create server_from_intermediate.key"
openssl genrsa -out server_from_intermediate.key ${BITS} || exit $?

log "Create server_from_intermediate.csr"
openssl req -new -sha256 -key server_from_intermediate.key \
  -subj "/C=US/ST=WA/O=Pistache/CN=server_from_intermediate" \
  -out server_from_intermediate.csr || exit $?

log "Create server_from_intermediate.crt"
openssl x509 -req -in server_from_intermediate.csr -days ${DAYS} -sha256 -extfile openssl.conf \
  -CA intermediateCA.crt -CAkey intermediateCA.key -set_serial 01 \
  -extensions server -out server_from_intermediate.crt || exit $?

rm -f server_from_intermediate.csr || exit $?

log "Create server_protected.key"
openssl genrsa -aes128 -passout pass:test -out server_protected.key ${BITS} || exit $?

log "Create server_protected.csr"
openssl req -new -sha256 -passin pass:test -key server_protected.key \
  -subj "/C=US/ST=WA/O=Pistache/CN=server" \
  -out server_protected.csr || exit $?

log "Create server_protected.crt"
openssl x509 -req -in server_protected.csr -days ${DAYS} -sha256 -extfile openssl.conf \
  -CA rootCA.crt -CAkey rootCA.key -set_serial 01 \
  -extensions server -out server_protected.crt || exit $?

rm -f server_protected.csr || exit $?

log "Verify server certificate"
openssl verify -purpose sslserver -CAfile rootCA.crt server.crt || exit $?

log "Verify client certificate"
openssl verify -purpose sslclient -CAfile rootCA.crt client.crt || exit $?

log "Verify server_from_intermediate certificate against intermediate"
openssl verify -purpose sslserver -partial_chain -CAfile intermediateCA.crt server_from_intermediate.crt || exit $?

log "Verify server_from_intermediate certificate against root using intermediate"
openssl verify -purpose sslserver -untrusted intermediateCA.crt -CAfile rootCA.crt server_from_intermediate.crt || exit $?

log "Verify server_protected certificate"
openssl verify -purpose sslserver -CAfile rootCA.crt server_protected.crt || exit $?

log "Create server_from_intermediate_with_chain.crt"
cat server_from_intermediate.crt intermediateCA.crt > server_from_intermediate_with_chain.crt

log "done"
