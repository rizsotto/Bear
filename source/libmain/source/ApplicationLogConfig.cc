/*  Copyright (C) 2012-2021 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "config.h"
#include "libmain/ApplicationLogConfig.h"

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif

namespace {

    struct PointerArray {
        const char **values;
    };

    std::ostream &operator<<(std::ostream &os, const PointerArray &arguments) {
        os << '[';
        for (const char **it = arguments.values; *it != nullptr; ++it) {
            if (it != arguments.values) {
                os << ", ";
            }
            os << '"' << *it << '"';
        }
        os << ']';

        return os;
    }
}

namespace ps {

    ApplicationLogConfig::ApplicationLogConfig(const char *name, const char *id)
            : name_(name)
            , id_(id)
    {
        spdlog::set_default_logger(spdlog::stderr_logger_mt("stderr"));
    }

    void ApplicationLogConfig::initForSilent() const
    {
        spdlog::set_pattern(fmt::format("{0}: %v", name_));
        spdlog::set_level(spdlog::level::info);
    }

    void ApplicationLogConfig::initForVerbose() const
    {
        spdlog::set_pattern(fmt::format("[%H:%M:%S.%f, {0}, %P] %v", id_));
        spdlog::set_level(spdlog::level::debug);
    }

    void ApplicationLogConfig::record(const char** argv, const char** envp) const
    {
        spdlog::debug("{0}: {1}", name_, cfg::VERSION);
        spdlog::debug("arguments: {0}", PointerArray { argv });
        spdlog::debug("environment: {0}", PointerArray { envp });
    }

    void ApplicationLogConfig::context() const {
#ifdef HAVE_UNAME
        auto name = utsname{};
        if (const int status = uname(&name); status >= 0) {
            spdlog::debug("sysname: {0}", name.sysname);
            spdlog::debug("release: {0}", name.release);
            spdlog::debug("version: {0}", name.version);
            spdlog::debug("machine: {0}", name.machine);
        }
        errno = 0;
#endif
    }
}
