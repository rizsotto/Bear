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

#include "Tool.h"
#include "ToolGcc.h"

#include <filesystem>
#include <functional>
#include <stdexcept>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace fs = std::filesystem;

namespace cs {

    Tools::Tools(ToolPtrs&& tools) noexcept
            : tools_(tools)
    { }

    rust::Result<Tools> Tools::from(const cfg::Compilation& cfg)
    {
        ToolPtrs tools = {
                std::make_shared<ToolGcc>(cfg.compilers),
        };
        return rust::Ok(Tools(std::move(tools)));
    }

    output::Entries Tools::transform(const report::Report& report) const
    {
        output::Entries result;
        for (const auto& execution : report.executions) {
            spdlog::debug("Checking [pid: {}], command: {}", execution.run.pid, execution.command);
            recognize(execution.command)
                    .on_success([&execution, &result](auto items) {
                        // copy to results if the config allows it
                        std::copy(items.begin(), items.end(), std::back_inserter(result));
                        spdlog::debug("Checking [pid: {}], Recognized as: [{}]", execution.run.pid, items);
                    })
                    .on_error([&execution](const auto& error) {
                        spdlog::debug("Checking [pid: {}], {}", execution.run.pid, error.what());
                    });
        }
        return result;
    }

    rust::Result<output::Entries> Tools::recognize(const report::Command& command) const
    {
        // check if any tool can recognize the command.
        for (const auto& tool : tools_) {
            // when the tool is matching...
            if (tool->recognize(command.program)) {
                // return the recognized compilations.
                return tool->compilations(command);
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this command."));
    }
}
