/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

namespace Pistache
{

#define REQUIRES(condition) typename std::enable_if<(condition), int>::type = 0

} // namespace Pistache
