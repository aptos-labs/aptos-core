# SPDX-FileCopyrightText: 2021 Andrea Pappacoda
#
# SPDX-License-Identifier: Apache-2.0

configure_file(input: 'version.h.in', output: 'version.h', configuration: version_conf, install: get_option('PISTACHE_INSTALL'), install_dir: get_option('includedir')/'pistache')

install_headers(
	'async.h',
	'base64.h',
	'client.h',
	'common.h',
	'config.h',
	'cookie.h',
	'description.h',
	'endpoint.h',
	'errors.h',
	'flags.h',
	'http_defs.h',
	'http.h',
	'http_header.h',
	'http_headers.h',
	'iterator_adapter.h',
	'listener.h',
	'log.h',
	'mailbox.h',
	'mime.h',
	'meta.h',
	'net.h',
	'os.h',
	'peer.h',
	'prototype.h',
	'reactor.h',
	'route_bind.h',
	'router.h',
	'ssl_wrappers.h',
	'stream.h',
	'string_logger.h',
	'tcp.h',
	'timer_pool.h',
	'transport.h',
	'type_checkers.h',
	'typeid.h',
	'utils.h',
	'view.h',
	subdir: 'pistache')

install_subdir('serializer', install_dir: get_option('includedir')/'pistache')
