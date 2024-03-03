# SPDX-FileCopyrightText: 2021 Andrea Pappacoda
#
# SPDX-License-Identifier: Apache-2.0

doxygen = find_program('doxygen')
dot = find_program('dot')
mscgen = find_program('mscgen', required: false)

configure_file(input: 'Doxyfile.in', output: 'Doxyfile', configuration: {
	'CURRENT_BUILD_DIR': meson.current_build_dir(),
	'SOURCE_DIR': meson.source_root(),
	'PROJECT_NAME': meson.project_name()
})

run_target('docs', command: [doxygen, meson.current_build_dir()/'Doxyfile'])
