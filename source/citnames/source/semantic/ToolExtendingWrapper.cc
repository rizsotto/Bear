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

#include "ToolExtendingWrapper.h"
#include "ToolGcc.h"

namespace cs {

    semantic::ToolExtendingWrapper::ToolExtendingWrapper(CompilerWrapper &&compilers_to_recognize) noexcept
            : compilers_to_recognize_(compilers_to_recognize)
    { }

    const char *semantic::ToolExtendingWrapper::name() const {
        return compilers_to_recognize_.executable.c_str();
    }

    bool semantic::ToolExtendingWrapper::recognize(const fs::path &program) const {
        return compilers_to_recognize_.executable == program;
    }

    rust::Result<cs::semantic::SemanticPtrs> semantic::ToolExtendingWrapper::compilations(const report::Command &command) const {
        return ToolGcc().compilations(command)
                .map<cs::semantic::SemanticPtrs>([this](auto semantics) {
                    for (auto& semantic : semantics) {
                        semantic->extend_flags(compilers_to_recognize_.additional_flags);
                    }
                    return semantics;
                });
    }
}
