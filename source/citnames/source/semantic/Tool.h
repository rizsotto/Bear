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

#pragma once

#include "Configuration.h"
#include "EventsDatabase.h"
#include "Output.h"
#include "semantic/Semantic.h"
#include "libresult/Result.h"

#include <filesystem>
#include <list>
#include <memory>

namespace fs = std::filesystem;

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

    // Represents an expert system which can recognize compilation entries from
    // command executions. It covers multiple tools and consider omit results
    // based on configuration.
    class Tools {
    public:
        Tools() = delete;
        ~Tools() noexcept = default;

        static rust::Result<Tools> from(Compilation cfg);

        [[nodiscard]]
        Entries transform(cs::EventsDatabase::Ptr events) const;

    private:
        explicit Tools(std::shared_ptr<Tool> tool) noexcept;

        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const Execution &execution, uint32_t pid) const;

    private:
        std::shared_ptr<Tool> tool_;
    };
}
