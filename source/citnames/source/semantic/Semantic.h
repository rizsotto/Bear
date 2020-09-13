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
#include "libreport/Report.h"

#include <filesystem>
#include <list>
#include <memory>
#include <optional>
#include <ostream>

namespace fs = std::filesystem;

namespace cs::semantic {

    // Represents a recognized command. Which we can find out the intent of
    // that command. And therefore we know the semantic of it.
    struct Semantic {
        explicit Semantic(report::Command) noexcept;
        virtual ~Semantic() noexcept = default;

        virtual void extend_flags(std::list<std::string> const&) = 0;

        virtual std::ostream& operator<<(std::ostream&) const = 0;
        [[nodiscard]] virtual std::optional<cs::Entry> into_entry() const = 0;

        report::Command command;
    };

    using SemanticPtr = std::shared_ptr<Semantic>;
    using SemanticPtrs = std::list<SemanticPtr>;

    // Represents a compiler call, which does process any input, but query
    // something from the compiler itself. It can be a help or a version query.
    struct QueryCompiler : public Semantic {
        explicit QueryCompiler(report::Command) noexcept;

        void extend_flags(std::list<std::string> const&) override;

        std::ostream& operator<<(std::ostream&) const override;
        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;
    };

    // Represents a compiler call, which runs the preprocessor pass.
    struct Preprocess : public Semantic {
        Preprocess(report::Command, fs::path source, fs::path output, std::list<std::string>) noexcept;

        void extend_flags(std::list<std::string> const&) override;

        std::ostream& operator<<(std::ostream&) const override;
        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        fs::path source;
        fs::path output;
        std::list<std::string> flags;
    };

    // Represents a compiler call, which runs the compilation pass.
    struct Compile : public Semantic {
        Compile(report::Command, fs::path source, fs::path output, std::list<std::string>) noexcept;

        void extend_flags(std::list<std::string> const&) override;

        std::ostream& operator<<(std::ostream&) const override;
        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        fs::path source;
        fs::path output;
        std::list<std::string> flags;
    };

    // Represents a compiler call, which runs the linking pass.
    struct Link : public Semantic {

        void extend_flags(std::list<std::string> const&) override;

        std::ostream& operator<<(std::ostream&) const override;
        [[nodiscard]] std::optional<cs::Entry> into_entry() const override;

        enum Type {
            EXECUTABLE,
            LIBRARY
        };

        std::list<fs::path> inputs;
        std::list<fs::path> libraries;
        fs::path output;
        Type type;
        std::list<std::string> flags;

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
}
