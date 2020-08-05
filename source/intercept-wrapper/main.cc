/*  Copyright (C) 2012-2020 by László Nagy
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

#include "Application.h"

#include "libwrapper/Environment.h"
#include "libsys/Os.h"

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>
#include <spdlog/sinks/stdout_sinks.h>

#include <iostream>

namespace {

    struct Arguments {
        char *const * values;
    };

    std::ostream& operator<<(std::ostream& os, const Arguments& arguments)
    {
        os << '[';
        for (char* const* it = arguments.values; *it != nullptr; ++it) {
            if (it != arguments.values) {
                os << ", ";
            }
            os << '"' << *it << '"';
        }
        os << ']';

        return os;
    }

    bool is_verbose()
    {
        return (nullptr != getenv(wr::env::KEY_VERBOSE));
    }
}

int main(int, char* argv[], char* envp[])
{
    spdlog::set_default_logger(spdlog::stderr_logger_mt("stderr"));
    spdlog::set_pattern(is_verbose() ? "[%H:%M:%S.%f, wr, %P] %v" : "wrapper: %v [pid: %P]");
    spdlog::set_level(is_verbose() ? spdlog::level::debug : spdlog::level::info);

    spdlog::debug("wrapper: {}", VERSION);
    spdlog::debug("arguments raw: {}", Arguments { argv });

    auto environment = sys::env::from(const_cast<const char **>(envp));
    return wr::Application::create(const_cast<const char**>(argv), std::move(environment))
        .and_then<int>([](const auto& command) {
            return command();
        })
        // print out the result of the run
        .on_error([](auto error) {
            spdlog::error("failed with: {}", error.what());
        })
        .on_success([](auto status_code) {
            spdlog::debug("succeeded with: {}", status_code);
        })
        // set the return code from error
        .unwrap_or(EXIT_FAILURE);
}
