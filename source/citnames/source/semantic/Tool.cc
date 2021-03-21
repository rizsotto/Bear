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

#include "Tool.h"
#include "ToolAny.h"
#include "ToolGcc.h"
#include "ToolClang.h"
#include "ToolCuda.h"
#include "ToolWrapper.h"
#include "ToolExtendingWrapper.h"
#include "Convert.h"

#include <filesystem>
#include <unordered_map>
#include <stdexcept>
#include <utility>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace fs = std::filesystem;

namespace cs::semantic {

    Tools::Tools(std::shared_ptr<Tool> tool) noexcept
            : tool_(std::move(tool))
    { }

    rust::Result<Tools> Tools::from(Compilation cfg) {
        // TODO: use `cfg.flags_to_remove`
        ToolAny::ToolPtrs tools = {
                std::make_shared<ToolGcc>(),
                std::make_shared<ToolClang>(),
                std::make_shared<ToolWrapper>(),
                std::make_shared<ToolCuda>(),
        };
        for (auto && compiler : cfg.compilers_to_recognize) {
            tools.emplace_back(std::make_shared<ToolExtendingWrapper>(std::move(compiler)));
        }
        std::shared_ptr<ToolAny> tool =
                std::make_shared<ToolAny>(std::move(tools), std::move(cfg.compilers_to_exclude));

        return rust::Ok(Tools(tool));
    }

    Entries Tools::transform(cs::EventsDatabase::Ptr events) const {
        Entries results;
        for (EventsIterator it = events->events_begin(), end = events->events_end(); it != end; ++it) {
            (*it).and_then<SemanticPtr>([this](const cs::EventPtr &event) {
                if (event->has_started()) {
                    auto execution = domain::from(event->started().execution());
                    auto pid = event->started().pid();
                    return recognize(execution, pid);
                } else {
                    return rust::Result<SemanticPtr>(rust::Err(std::runtime_error("other")));
                }
            })
            .on_success([&results](const auto &semantic) {
                auto candidate = dynamic_cast<const CompilerCall*>(semantic.get());
                if (candidate != nullptr) {
                    auto entries = candidate->into_entries();
                    std::copy(entries.begin(), entries.end(), std::back_inserter(results));
                }
            });
        }
        return results;
    }

    [[nodiscard]]
    rust::Result<SemanticPtr> Tools::recognize(const Execution &execution, const uint32_t pid) const {
        spdlog::debug("[pid: {}] execution: {}", pid, execution);

        auto result = tool_->recognize(execution);
        if (Tool::recognized_ok(result)) {
            spdlog::debug("[pid: {}] recognized.", pid);
        } else if (Tool::recognized_with_error(result)) {
            spdlog::debug("[pid: {}] recognition failed: {}", pid, result.unwrap_err().what());
        } else if (Tool::not_recognized(result)) {
            spdlog::debug("[pid: {}] not recognized.", pid);
        }
        return result;
    }
}
