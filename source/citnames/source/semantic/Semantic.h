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

#include "Output.h"
#include "Domain.h"

#include <filesystem>
#include <list>
#include <memory>
#include <optional>
#include <ostream>
#include <utility>

namespace fs = std::filesystem;

namespace cs::semantic {

    using namespace domain;

    // Represents a recognized command. Which we can find out the intent
    // of that command. And therefore we know the semantic of it.
    struct Semantic {
        virtual ~Semantic() noexcept = default;

        virtual bool operator==(Semantic const&) const = 0;
        virtual std::ostream& operator<<(std::ostream&) const = 0;
    };

    inline
    std::ostream& operator<<(std::ostream& os, Semantic const& value) {
        value.operator<<(os);
        return os;
    }

    inline
    bool operator==(Semantic const &lhs, Semantic const &rhs) {
        return lhs.operator==(rhs);
    }

    using SemanticPtr = std::shared_ptr<Semantic>;

    // Represents a compiler call command.
    struct CompilerCall : public Semantic {
        [[nodiscard]] virtual cs::Entries into_entries() const = 0;
    };

    // Represents a compiler call, which does process any input, but query
    // something from the compiler itself. It can be a help or a version query.
    struct QueryCompiler : public CompilerCall {
        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] cs::Entries into_entries() const override;
    };

    // Represents a compiler call, which runs only the preprocessor.
    struct Preprocess : public CompilerCall {

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] cs::Entries into_entries() const override;
    };

    // Represents a compiler call, which runs the compilation pass.
    struct Compile : public CompilerCall {
        Compile(fs::path working_dir,
                fs::path compiler,
                std::vector<std::string> flags,
                std::vector<fs::path> sources,
                std::optional<fs::path> output);

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] cs::Entries into_entries() const override;

    public:
        fs::path working_dir;
        fs::path compiler;
        std::vector<std::string> flags;
        std::vector<fs::path> sources;
        std::optional<fs::path> output;
    };
}
