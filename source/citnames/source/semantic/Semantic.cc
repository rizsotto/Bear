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

namespace {

    inline
    fs::path make_absolute(report::Command const& command, fs::path && path) {
        return (path.is_absolute()) ? path : command.working_dir / path;
    }
}

namespace cs::semantic {

    Semantic::Semantic(report::Command _command) noexcept
            : command(std::move(_command))
    { }

    QueryCompiler::QueryCompiler(report::Command _command) noexcept
            : Semantic(std::move(_command))
    { }

    Preprocess::Preprocess(report::Command _command, fs::path _source, fs::path _output, std::list<std::string> _flags) noexcept
            : Semantic(std::move(_command))
            , source(make_absolute(command, std::move(_source)))
            , output(make_absolute(command, std::move(_output)))
            , flags(std::move(_flags))
    { }

    Compile::Compile(report::Command _command, fs::path _source, fs::path _output, std::list<std::string> _flags) noexcept
            : Semantic(std::move(_command))
            , source(make_absolute(command, std::move(_source)))
            , output(make_absolute(command, std::move(_output)))
            , flags(std::move(_flags))
    { }

    void QueryCompiler::extend_flags(const std::list<std::string> &) {
    }

    void Preprocess::extend_flags(const std::list<std::string> &_flags) {
        std::copy(_flags.begin(), _flags.end(), std::back_inserter(flags));
    }

    void Compile::extend_flags(const std::list<std::string> &_flags) {
        std::copy(_flags.begin(), _flags.end(), std::back_inserter(flags));
    }

    void Link::extend_flags(const std::list<std::string> &_flags) {
        std::copy(_flags.begin(), _flags.end(), std::back_inserter(flags));
    }

    std::ostream &QueryCompiler::operator<<(std::ostream &os) const {
        os << "Query";
        return os;
    }

    std::ostream &Preprocess::operator<<(std::ostream &os) const {
        os << "Preprocess";
        return os;
    }

    std::ostream &Compile::operator<<(std::ostream &os) const {
        os << "Compile";
        return os;
    }

    std::ostream &Link::operator<<(std::ostream &os) const {
        os << "Link";
        return os;
    }

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

    std::optional<cs::Entry> Link::into_entry() const {
        // TODO
        return std::optional<cs::Entry>();
    }
}