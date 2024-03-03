# SPDX-FileCopyrightText: 2021 Andrea Pappacoda
#
# SPDX-License-Identifier: Apache-2.0

project(
	'pistache',
	'cpp',
	version: '0.1.1', # In Meson 0.57 this can be replaced with files('version.txt')
	license: 'Apache-2.0',
	default_options: [
		'cpp_std=c++17',
		'buildtype=release',
		'b_ndebug=if-release',
		'b_lto=false',
		'warning_level=3'
	],
	meson_version: '>=0.50.0'
)

if host_machine.system() != 'linux'
	error('Pistache currenly only supports Linux. See https://github.com/pistacheio/pistache/issues/6#issuecomment-242398225 for more information')
endif

compiler = meson.get_compiler('cpp')

# Wrapping arguments inside a call to get_supported_arguments so that only supported arguments get applied
# No need for -Wall -Wextra -Wpedantic, since warning_level is 3
add_project_arguments(compiler.get_supported_arguments(['-Wconversion', '-Wno-sign-conversion', '-Wno-missing-field-initializers']), language: 'cpp')

# No need for --coverage, since b_coverage is set
if get_option('b_coverage')
	add_project_arguments(compiler.get_supported_arguments(['-fstack-protector-all', '--param=ssp-buffer-size=4']), language: 'cpp')
endif

# howardhinnant/date has several names - look for them, from the most
# to the least explicit name.
# In Meson 0.60.0, this can be replaced with a simpler:
#
#     dependency('howardhinnant-date', 'hinnant-date', 'date')
#
date_dep = dependency('howardhinnant-date', required: false)
if not date_dep.found()
	date_dep = dependency('hinnant-date', required: false)
endif
if not date_dep.found()
	date_dep = dependency('date', fallback: ['hinnant-date', 'date_dep'])
endif

deps_libpistache = [
	dependency('threads'),
	dependency('RapidJSON', fallback: ['rapidjson', 'rapidjson_dep']),
	date_dep
]

# Check if -latomic is needed - https://github.com/llvm/llvm-project/blob/main/llvm/cmake/modules/CheckAtomic.cmake
compiler_id = compiler.get_id()

cxx_atomics_check_code = '''
#include <atomic>
std::atomic<int> x;
std::atomic<short> y;
std::atomic<char> z;
int main() {
	++z;
	++y;
	return ++x;
}
'''
has_working_cxx_atomics = compiler.links(cxx_atomics_check_code, name: 'std::atomic')
if (compiler_id == 'clang' or compiler_id == 'gcc') and not has_working_cxx_atomics
	libatomic_dep = compiler.find_library('atomic')
	has_working_cxx_atomics = compiler.links(cxx_atomics_check_code, dependencies: libatomic_dep, name: 'std::atomic with libatomic')
	assert(has_working_cxx_atomics, 'Host compiler must support std::atomic')
	deps_libpistache += libatomic_dep
endif

cxx_atomics64_check_code = '''
#include <atomic>
#include <cstdint>
std::atomic<uint64_t> x (0);
int main() {
	uint64_t i = x.load(std::memory_order_relaxed);
	(void)i;
	return 0;
}
'''
has_working_cxx_atomics64 = compiler.links(cxx_atomics64_check_code, name: 'std::atomic<uint64_t>')
if (compiler_id == 'clang' or compiler_id == 'gcc') and not has_working_cxx_atomics64
	libatomic_dep = compiler.find_library('atomic')
	has_working_cxx_atomics = compiler.links(cxx_atomics64_check_code, dependencies: libatomic_dep, name: 'std::atomic<uint64_t> with libatomic')
	assert(has_working_cxx_atomics, 'Host compiler must support 64-bit std::atomic')
	deps_libpistache += libatomic_dep
endif

# Workaround https://github.com/pistacheio/pistache/issues/1068
if compiler_id == 'gcc' and compiler.version().version_compare('<9.1') or compiler_id == 'clang'
	cpp_fs_dep = compiler.find_library('stdc++fs', required: false)
	if not cpp_fs_dep.found()
		cpp_fs_dep = compiler.find_library('c++fs', required: false)
	endif
	if cpp_fs_dep.found()
		deps_libpistache += cpp_fs_dep
	endif
endif

if get_option('PISTACHE_USE_SSL')
	deps_libpistache += dependency('openssl')
endif

version_array = []
if meson.version().version_compare('>=0.57.0')
	version_array = import('fs').read('version.txt').strip().split('.')
else
	# Ugly workaround for reading a file
	version_array = run_command(
		find_program('python3'), '-c', 'print(open("version.txt").read())',
		check: true
	).stdout().strip().split('.')
endif

version_major    = version_array[0]
version_minor    = version_array[1]
version_patch    = version_array[2]
version_git_date = version_array[3]
version_str      = '@0@.@1@.@2@'.format(version_major, version_minor, version_patch)

version_conf = configuration_data()
version_conf.set('VERSION_MAJOR',    version_major)
version_conf.set('VERSION_MINOR',    version_minor)
version_conf.set('VERSION_PATCH',    version_patch)
version_conf.set('VERSION_GIT_DATE', version_git_date)

incl_pistache = include_directories('include')

subdir('include'/'pistache')
subdir('src')
if get_option('PISTACHE_BUILD_TESTS')
	subdir('tests')
endif
if get_option('PISTACHE_BUILD_EXAMPLES')
	subdir('examples')
endif
if get_option('PISTACHE_BUILD_DOCS')
	subdir('docs')
endif

if not meson.is_subproject()
	git = find_program('git', required: false)
	if git.found()
		run_command(git, 'config', '--local', 'core.hooksPath', meson.source_root()/'.hooks', check: false)
	endif
endif
