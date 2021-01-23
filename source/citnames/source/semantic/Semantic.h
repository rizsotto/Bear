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

#pragma once

#include "Output.h"
#include "Domain.h"

#include <filesystem>
#include <list>
#include <memory>
#include <optional>
#include <ostream>

namespace fs = std::filesystem;

namespace cs::semantic {

    using namespace domain;

    // Represents a recognized command. Which we can find out the intent of
    // that command. And therefore we know the semantic of it.
    struct Semantic {
        explicit Semantic(Execution) noexcept;
        virtual ~Semantic() noexcept = default;

        [[nodiscard]] virtual std::optional<cs::Entry> into_entry() const = 0;

        virtual bool operator==(Semantic const&) const = 0;
        virtual std::ostream& operator<<(std::ostream&) const = 0;

        Execution execution;
    };

    using SemanticPtr = std::shared_ptr<Semantic>;
    using SemanticPtrs = std::list<SemanticPtr>;

    // Represents a compiler call, which does process any input, but query
    // something from the compiler itself. It can be a help or a version query.
    struct QueryCompiler : public Semantic {
        explicit QueryCompiler(Execution) noexcept;

        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;
    };

    // Represents a compiler call, which runs the preprocessor pass.
    struct Preprocess : public Semantic {
        Preprocess(Execution, fs::path source, fs::path output, std::vector<std::string>) noexcept;

        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        fs::path source;
        fs::path output;
        std::vector<std::string> flags;
    };

    // Represents a compiler call, which runs the compilation pass.
    struct Compile : public Semantic {
        Compile(Execution, fs::path source, fs::path output, std::vector<std::string>) noexcept;

        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        fs::path source;
        fs::path output;
        std::vector<std::string> flags;
    };

    inline
    std::ostream& operator<<(std::ostream& os, Semantic const& value) {
        value.operator<<(os);
        return os;
    }

    inline
    std::ostream& operator<<(std::ostream& os, SemanticPtrs const& values) {
        for (const auto& value : values) {
            if (values.front().get() != value.get()) {
                os << ", ";
            }
            value->operator<<(os);
        }
        return os;
    }

    inline
    bool operator==(Semantic const &lhs, Semantic const &rhs) {
        return lhs.operator==(rhs);
    }

    inline
    bool operator==(SemanticPtrs const &lhs, SemanticPtrs const &rhs) {
        auto lhs_it = lhs.begin();
        auto rhs_it = rhs.begin();
        const auto lhs_end = lhs.end();
        const auto rhs_end = rhs.end();
        while (lhs_it != lhs_end && rhs_it != rhs_end && (*lhs_it)->operator==(**rhs_it)) {
            ++lhs_it;
            ++rhs_it;
        }
        return lhs_it == lhs_end && rhs_it == rhs_end;
    }
}
