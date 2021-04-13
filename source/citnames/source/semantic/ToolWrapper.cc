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

#include "ToolWrapper.h"

#include <set>

namespace {

    bool looks_like_ccache_parameter(const std::string &candidate) {
        return candidate.empty() || candidate[0] == '-';
    }

    bool looks_like_distcc_parameter(const std::string &candidate) {
        static std::set<std::string_view> flags = {
                "--help", "--version", "--show-hosts", "--scan-includes", "-j", "--show-principal"
        };
        return candidate.empty() || (flags.find(candidate) != flags.end());
    }

    rust::Result<domain::Execution> remove_wrapper(const domain::Execution &execution) {
        domain::Execution result = execution;
        result.arguments.pop_front();
        result.executable = result.arguments.front(); // FIXME
        return rust::Ok(result);
    }
}

namespace cs::semantic {

    rust::Result<SemanticPtr> ToolWrapper::recognize(const Execution &execution) const {
        if (is_ccache_call(execution.executable)) {
            return is_ccache_query(execution.arguments)
                    ? rust::Ok<SemanticPtr>(std::make_shared<QueryCompiler>())
                    : remove_wrapper(execution)
                            .and_then<SemanticPtr>([this](const auto &wrapped_execution) {
                                return compilation(wrapped_execution);
                            });
        }
        if (is_distcc_call(execution.executable)) {
            return is_distcc_query(execution.arguments)
                    ? rust::Ok<SemanticPtr>(std::make_shared<QueryCompiler>())
                    : remove_wrapper(execution)
                            .and_then<SemanticPtr>([this](const auto &wrapped_execution) {
                                return compilation(wrapped_execution);
                            });
        }
        return rust::Ok(SemanticPtr());
    }

    bool ToolWrapper::is_ccache_call(const fs::path &program) {
        const auto string = program.filename().string();
        return string == "ccache";
    }

    bool ToolWrapper::is_ccache_query(const std::list<std::string> &arguments) {
        if (arguments.size() <= 1) {
            return true;
        }
        if (auto second = std::next(arguments.begin()); looks_like_ccache_parameter(*second)) {
            return true;
        }
        return false;
    }

    bool ToolWrapper::is_distcc_call(const fs::path &program) {
        const auto string = program.filename().string();
        return string == "distcc";
    }

    bool ToolWrapper::is_distcc_query(const std::list<std::string> &arguments) {
        if (arguments.size() <= 1) {
            return true;
        }
        if (auto second = std::next(arguments.begin()); looks_like_distcc_parameter(*second)) {
            return true;
        }
        return false;
    }
}
