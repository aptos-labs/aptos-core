# SPDX-FileCopyrightText: 2021 Andrea Pappacoda
#
# SPDX-License-Identifier: Apache-2.0

curl_dep = dependency('libcurl')
gtest_main_dep = dependency('gtest', main: true, fallback: ['gtest', 'gtest_main_dep'])
gmock_dep = dependency('gmock', version: '>=1.11.0', fallback: ['gtest', 'gmock_dep'])
cpp_httplib_dep = dependency('cpp-httplib', fallback: ['cpp-httplib', 'cpp_httplib_dep'])

pistache_test_files = [
	'async_test',
	'cookie_test',
	'cookie_test_2',
	'cookie_test_3',
	'headers_test',
	'http_client_test',
	'http_parsing_test',
	'http_server_test',
	'http_uri_test',
	'listener_test',
	'log_api_test',
	'mailbox_test',
	'mime_test',
	'net_test',
	'reactor_test',
	'request_size_test',
	'rest_server_test',
	'rest_swagger_server_test',
	'router_test',
	'stream_test',
	'streaming_test',
	'string_logger_test',
	'threadname_test',
	'typeid_test',
	'view_test',
]

network_tests = ['net_test']

flaky_tests = []

if get_option('PISTACHE_USE_SSL')
	pistache_test_files += [
		'https_server_test',
		'listener_tls_test'
	]
	subdir('certs')
endif

foreach test_name : pistache_test_files
	suite = {}
	if test_name in network_tests
		suite = {'suite': 'network'}
	elif test_name in flaky_tests
		suite = {'suite': 'flaky'}
	endif

	test(
		test_name,
		executable(
			'run_'+test_name,
			test_name+'.cc',
			dependencies: [
				pistache_dep,
				deps_libpistache,
				gtest_main_dep,
				gmock_dep,
				curl_dep,
				cpp_httplib_dep
			]
		),
		timeout: 600,
		workdir: meson.current_build_dir(),
		is_parallel: false,
		kwargs: suite
	)
endforeach

cppcheck = find_program('cppcheck', required: false)
if cppcheck.found()
	cppcheck_args = [
		'--project=' + (meson.build_root()/'compile_commands.json'),
		'--enable=all',
		'-DCPPCHECK',
		'--suppress=*:*tests/*',
		'--suppress=*:*subprojects/*',
		'--std=' + get_option('cpp_std')
	]
	run_target(
		'cppcheck',
		command: [
			cppcheck,
			cppcheck_args
		]
	)
endif
