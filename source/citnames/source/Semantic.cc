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

#include "Semantic.h"
#include "libsys/Path.h"

#include <spdlog/spdlog.h>

namespace cs {

    Semantic::Semantic(const cfg::Value& config, const sys::Context& ctx, Tools && tools) noexcept
            : config_(config)
            , ctx_(ctx)
            , tools_(tools)
    { }

    rust::Result<Semantic> Semantic::from(const cfg::Value& cfg, const sys::Context& ctx)
    {
        Tools tools = {
                std::make_shared<GnuCompilerCollection>(cfg.compilation.compilers),
        };
        return rust::Ok(Semantic(cfg, ctx, std::move(tools)));
    }

    output::Entries Semantic::transform(const report::Report& report) const
    {
        output::Entries result;
        for (const auto& execution : report.executions) {
            //spdlog::debug("checking: {}", execution.command.arguments);
            if (auto entries = recognize(execution.command); entries.is_ok()) {
                entries.on_success([this, &result](auto items) {
                    // copy to results if the config allows it
                    std::copy_if(items.begin(), items.end(),
                            std::back_inserter(result),
                            [this](auto entry) { return filter(entry); });
                });
            }
        }
        return result;
    }

    rust::Result<output::Entries> Semantic::recognize(const report::Execution::Command& command) const
    {
        // check if any tool can recognize the command.
        for (const auto& tool : tools_) {
            // the first it recognize it won't seek for more.
            if (auto semantic = tool->recognize(command); semantic.is_ok()) {
                return semantic;
            }
        }
        return rust::Err(std::runtime_error("No tools recognize this command."));
    }

    [[nodiscard]]
    bool Semantic::filter(const output::Entry &entry) const
    {
        const auto &exclude = config_.content.paths_to_exclude;
        const bool to_exclude = (std::find_if(exclude.begin(), exclude.end(),
                                              [&entry](auto directory) {
                                                  return sys::path::contains(directory, entry.file);
                                              }) !=
                                 exclude.end());
        const bool exists = ctx_.is_exists(entry.file);

        return exists && !to_exclude;
    }
}
