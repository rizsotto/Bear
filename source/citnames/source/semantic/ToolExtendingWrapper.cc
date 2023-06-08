/*  Copyright (C) 2012-2023 by László Nagy
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

#include <algorithm>

namespace cs::semantic {

    ToolExtendingWrapper::ToolExtendingWrapper(CompilerWrapper &&compilers_to_recognize) noexcept
            : compilers_to_recognize_(compilers_to_recognize)
    { }

    bool ToolExtendingWrapper::is_compiler_call(const fs::path &program) const {
        return compilers_to_recognize_.executable == program;
    }

    rust::Result<SemanticPtr> ToolExtendingWrapper::recognize(const Execution &execution) const {
        return ToolGcc::recognize(execution)
                .map<cs::semantic::SemanticPtr>([this](auto semantic) {
                    if (auto* ptr = dynamic_cast<Compile*>(semantic.get()); ptr != nullptr) {
                        // remove flags which were asked to be removed.
                        ptr->flags.erase(
                                std::remove_if(
                                        ptr->flags.begin(),
                                        ptr->flags.end(),
                                        [this](auto flag) {
                                            return std::any_of(
                                                    compilers_to_recognize_.flags_to_remove.begin(),
                                                    compilers_to_recognize_.flags_to_remove.end(),
                                                    [&flag](auto flag_to_remove) { return flag_to_remove == flag; });
                                        }),
                                ptr->flags.end());
                        // add flags which were asked to be added.
                        std::copy(compilers_to_recognize_.flags_to_add.begin(),
                                  compilers_to_recognize_.flags_to_add.end(),
                                  std::back_inserter(ptr->flags));
                    }
                    return semantic;
                });
    }
}
