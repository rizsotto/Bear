/*  Copyright (C) 2012-2022 by László Nagy
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

#pragma once

#include "semantic/Semantic.h"
#include "libresult/Result.h"

#include <memory>

namespace cs::semantic {

    // Represents a program, which can recognize the intent of the execution
    // and return the semantic of that. It can be a compiler or any other
    // program participating in a build process.
    struct Tool {
        virtual ~Tool() noexcept = default;

        // Returns the semantic of a command execution.
        [[nodiscard]]
        virtual rust::Result<SemanticPtr> recognize(const Execution &) const = 0;

        // Helper methods to evaluate the recognize method result.
        static bool recognized_ok(const rust::Result<SemanticPtr> &result) noexcept;
        static bool recognized_with_error(const rust::Result<SemanticPtr> &result) noexcept;
        static bool not_recognized(const rust::Result<SemanticPtr> &result) noexcept;
    };

    inline
    bool Tool::recognized_ok(const rust::Result<SemanticPtr> &result) noexcept {
        return result.is_ok() && (result.unwrap().operator bool());
    }

    inline
    bool Tool::recognized_with_error(const rust::Result<SemanticPtr> &result) noexcept {
        return result.is_err();
    }

    inline
    bool Tool::not_recognized(const rust::Result<SemanticPtr> &result) noexcept {
        return result.is_ok() && !(result.unwrap().operator bool());
    }
}
