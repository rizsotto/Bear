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

#include "semantic/Semantic.h"

#include <fmt/format.h>

namespace {

    inline
    fs::path make_absolute(cs::semantic::Command const& command, fs::path && path) {
        return (path.is_absolute()) ? path : command.working_dir / path;
    }
}

namespace cs::semantic {

    Semantic::Semantic(Command _command) noexcept
            : command(std::move(_command))
    { }

    QueryCompiler::QueryCompiler(Command _command) noexcept
            : Semantic(std::move(_command))
    { }

    Preprocess::Preprocess(Command _command, fs::path _source, fs::path _output, std::list<std::string> _flags) noexcept
            : Semantic(std::move(_command))
            , source(make_absolute(command, std::move(_source)))
            , output(make_absolute(command, std::move(_output)))
            , flags(std::move(_flags))
    { }

    Compile::Compile(Command _command, fs::path _source, fs::path _output, std::list<std::string> _flags) noexcept
            : Semantic(std::move(_command))
            , source(make_absolute(command, std::move(_source)))
            , output(make_absolute(command, std::move(_output)))
            , flags(std::move(_flags))
    { }

    std::optional<cs::Entry> QueryCompiler::into_entry() const {
        return std::optional<cs::Entry>();
    }

    std::optional<cs::Entry> Preprocess::into_entry() const {
        // TODO
        return std::optional<cs::Entry>();
    }

    std::optional<cs::Entry> Compile::into_entry() const {
        auto entry = cs::Entry {
                source,
                command.working_dir,
                std::make_optional(output),
                flags
        };
        return std::make_optional(std::move(entry));
    }

    bool QueryCompiler::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto* ptr = dynamic_cast<QueryCompiler const*>(&rhs); ptr != nullptr) {
            return (command == ptr->command);
        }
        return false;
    }

    bool Preprocess::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto* ptr = dynamic_cast<Preprocess const*>(&rhs); ptr != nullptr) {
            return (command == ptr->command) &&
                   (source == ptr->source) &&
                   (output == ptr->output) &&
                   (flags == ptr->flags);
        }
        return false;
    }

    bool Compile::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto* ptr = dynamic_cast<Compile const*>(&rhs); ptr != nullptr) {
            return (command == ptr->command) &&
                   (source == ptr->source) &&
                   (output == ptr->output) &&
                   (flags == ptr->flags);
        }
        return false;
    }

    std::ostream &QueryCompiler::operator<<(std::ostream &os) const {
        os << "Query";
        return os;
    }

    std::ostream &Preprocess::operator<<(std::ostream &os) const {
        os  << "Preprocess { source: " << source
            << ", output: " << output
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << " }";
        return os;
    }

    std::ostream &Compile::operator<<(std::ostream &os) const {
        os  << "Compile { source: " << source
            << ", output: " << output
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << " }";
        return os;
    }
}