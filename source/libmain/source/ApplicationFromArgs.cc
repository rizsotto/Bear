/*  Copyright (C) 2012-2024 by László Nagy
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

#include "libmain/ApplicationFromArgs.h"

#include <fmt/ostream.h>
#include <spdlog/spdlog.h>
#include <spdlog/sinks/stdout_sinks.h>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
template <> struct fmt::formatter<flags::Arguments> : ostream_formatter {};
#endif

namespace ps {

    ApplicationFromArgs::ApplicationFromArgs(const ApplicationLogConfig &log_config) noexcept
            : Application()
            , log_config_(log_config)
    {
        log_config_.initForSilent();
    }

    rust::Result<CommandPtr> ApplicationFromArgs::command(int argc, const char** argv, const char** envp) const
    {
        return parse(argc, argv)
            .on_success([this, &argv, &envp](const auto& args) {
                if (args.as_bool(flags::VERBOSE).unwrap_or(false)) {
                    log_config_.initForVerbose();
                }
                log_config_.record(argv, envp);
                log_config_.context();
                spdlog::debug("arguments parsed: {0}", args);
            })
            // if parsing success, we create the main command and execute it.
            .and_then<CommandPtr>([this, &envp](auto args) {
                return this->command(args, envp);
            });
    }
}
