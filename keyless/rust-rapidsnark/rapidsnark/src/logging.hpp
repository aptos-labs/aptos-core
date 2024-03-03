#ifndef LOGGING_HPP
#define LOGGING_HPP

#ifdef USE_LOGGER

#include "logger.hpp"

using namespace CPlusPlusLogging;

#else

#define LOG_ERROR(x)
#define LOG_ALARM(x)
#define LOG_ALWAYS(x)
#define LOG_INFO(x)
#define LOG_BUFFER(x)
#define LOG_TRACE(x)
#define LOG_DEBUG(x)

#endif // USE_LOGGER

#endif // LOGGING_HPP
