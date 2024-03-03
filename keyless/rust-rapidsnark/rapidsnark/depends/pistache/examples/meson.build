# SPDX-FileCopyrightText: 2021 Andrea Pappacoda
#
# SPDX-License-Identifier: Apache-2.0

pistache_example_files = [
	'custom_header',
	'hello_server',
	'http_client',
	'http_server_shutdown',
	'http_server',
	'rest_server',
	'rest_description'
]

threads_dep = dependency('threads')

foreach example_name : pistache_example_files
	executable('run'+example_name, example_name+'.cc', dependencies: [pistache_dep, threads_dep])
endforeach
