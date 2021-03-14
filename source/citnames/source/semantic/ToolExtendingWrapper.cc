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

#include "ToolExtendingWrapper.h"

namespace cs::semantic {

    ToolExtendingWrapper::ToolExtendingWrapper(CompilerWrapper &&compilers_to_recognize) noexcept
            : compilers_to_recognize_(compilers_to_recognize)
    { }

    bool ToolExtendingWrapper::recognize(const fs::path &program) const {
        return compilers_to_recognize_.executable == program;
    }

    rust::Result<SemanticPtrs> ToolExtendingWrapper::recognize(const Execution &execution) const {
        return ToolGcc::recognize(execution)
                .map<cs::semantic::SemanticPtrs>([this](auto semantics) {
                    for (auto& semantic : semantics) {
                        if (auto* ptr = dynamic_cast<Preprocess*>(semantic.get()); ptr != nullptr) {
                            std::copy(compilers_to_recognize_.additional_flags.begin(),
                                      compilers_to_recognize_.additional_flags.end(),
                                      std::back_inserter(ptr->flags));
                        }
                        if (auto* ptr = dynamic_cast<Compile*>(semantic.get()); ptr != nullptr) {
                            std::copy(compilers_to_recognize_.additional_flags.begin(),
                                      compilers_to_recognize_.additional_flags.end(),
                                      std::back_inserter(ptr->flags));
                        }
                    }
                    return semantics;
                });
    }
}
