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

#include "ToolCuda.h"
#include "ToolGcc.h"
#include "Parsers.h"

#include "libsys/Path.h"

#include <regex>
#include <utility>
#include <functional>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

using namespace cs::parser;

namespace {

    bool match_executable_name(const fs::path& program)
    {
        static const auto pattern = std::regex(R"(^(nvcc)$)");

        auto basename = program.filename();
        std::cmatch m;
        return std::regex_match(basename.c_str(), m, pattern);
    }
}

namespace cs {

    ToolCuda::ToolCuda()
            : Tool()
    { }

    bool ToolCuda::recognize(const fs::path& program) const {
        return match_executable_name(program);
    }

    rust::Result<output::Entries> ToolCuda::compilations(const report::Command &command) const {
        spdlog::debug("Recognized as a CudaCompiler execution.");
        std::list<fs::path> paths;
        auto tool = std::make_unique<ToolGcc>(paths);
        return tool->compilations(command);
    }
}
