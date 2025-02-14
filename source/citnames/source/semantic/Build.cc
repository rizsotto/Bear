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

#include "Build.h"
#include "ToolAny.h"
#include "ToolGcc.h"
#include "ToolClang.h"
#include "ToolCuda.h"
#include "ToolIntelFortran.h"
#include "ToolLinker.h"
#include "ToolWrapper.h"
#include "ToolAr.h"
#include "ToolExtendingWrapper.h"
#include "Convert.h"

#include <memory>
#include <utility>

#include <fmt/ostream.h>
#include <spdlog/spdlog.h>

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
template <> struct fmt::formatter<domain::Execution> : ostream_formatter {};
#endif

namespace {

    std::shared_ptr<cs::semantic::Tool> from(cs::Compilation cfg) {
        cs::semantic::ToolAny::ToolPtrs tools = {
                std::make_shared<cs::semantic::ToolGcc>(),
                std::make_shared<cs::semantic::ToolClang>(),
                std::make_shared<cs::semantic::ToolWrapper>(),
                std::make_shared<cs::semantic::ToolCuda>(),
                std::make_shared<cs::semantic::ToolIntelFortran>(),
                std::make_shared<cs::semantic::ToolLinker>(),
                std::make_shared<cs::semantic::ToolAr>(),
        };
        for (auto && compiler : cfg.compilers_to_recognize) {
            tools.emplace_front(std::make_shared<cs::semantic::ToolExtendingWrapper>(std::move(compiler)));
        }
        return std::make_shared<cs::semantic::ToolAny>(
                std::move(tools),
                std::move(cfg.compilers_to_exclude)
        );
    }
}

namespace cs::semantic {

    Build::Build(Compilation cfg) noexcept
            : tools_(from(std::move(cfg)))
    { }

    [[nodiscard]]
    rust::Result<SemanticPtr> Build::recognize(const rpc::Event &event) const {
        if (event.has_started()) {
            auto execution = domain::from(event.started().execution());
            auto pid = event.started().pid();

            spdlog::debug("[pid: {}] execution: {}", pid, execution);

            auto result = tools_->recognize(execution);
            if (Tool::recognized_ok(result)) {
                spdlog::debug("[pid: {}] recognized.", pid);
            } else if (Tool::recognized_with_error(result)) {
                spdlog::debug("[pid: {}] recognition failed: {}", pid, result.unwrap_err().what());
            } else if (Tool::not_recognized(result)) {
                spdlog::debug("[pid: {}] not recognized.", pid);
            }
            return result;
        } else {
            return rust::Err(std::runtime_error("n/a"));
        }
    }
}
