/*  Copyright (C) 2023 by Samu698
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

#include "libmain/SubcommandFromArgs.h"

#include <fmt/ostream.h>
#include <spdlog/spdlog.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <stdexcept>
#include <iostream>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
template <> struct fmt::formatter<flags::Arguments> : ostream_formatter {};
#endif

namespace ps {

	SubcommandFromArgs::SubcommandFromArgs(const char* name, const ApplicationLogConfig &log_config) noexcept
			: Subcommand()
			, name_(name)
			, log_config_(log_config)
	{
		log_config_.initForSilent();
	}

    bool SubcommandFromArgs::matches(const flags::Arguments &args) {
        return args.as_string(flags::COMMAND).unwrap_or("") == name_;
    }

	rust::Result<CommandPtr> SubcommandFromArgs::subcommand(const flags::Arguments &args, const char** envp) const {
        if (args.as_bool(flags::VERBOSE).unwrap_or(false)) {
            log_config_.initForVerbose();
        }

        return this->command(args, envp);
	}

}
