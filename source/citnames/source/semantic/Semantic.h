/*  Copyright (C) 2012-2024 by László Nagy
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
#include <vector>

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
        [[nodiscard]] virtual std::list<cs::Entry> into_entries() const = 0;
    };

    // Represents a compiler call, which does process any input, but query
    // something from the compiler itself. It can be a help or a version query.
    struct QueryCompiler : public CompilerCall {
        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] std::list<cs::Entry> into_entries() const override;
    };

    // Represents a compiler call, which runs only the preprocessor.
    struct Preprocess : public CompilerCall {

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] std::list<cs::Entry> into_entries() const override;
    };

    // Represents a compiler call, which runs the compilation pass.
    struct Compile : public CompilerCall {
        Compile(fs::path working_dir,
                fs::path compiler,
                std::list<std::string> flags,
                std::vector<fs::path> sources,
                std::optional<fs::path> output);

        bool operator==(Semantic const&) const override;
        std::ostream& operator<<(std::ostream&) const override;

        [[nodiscard]] std::list<cs::Entry> into_entries() const override;

    public:
        fs::path working_dir;
        fs::path compiler;
        std::list<std::string> flags;
        std::vector<fs::path> sources;
        std::optional<fs::path> output;
    };

    class Link : public CompilerCall {
    public:
        Link(fs::path working_dir,
             fs::path linker,
             std::list<std::string> flags,
             std::list<fs::path> input_files,
             std::optional<fs::path> output = std::nullopt)
            : working_dir(std::move(working_dir))
            , linker(std::move(linker))
            , flags(std::move(flags))
            , input_files(std::move(input_files))
            , output(std::move(output))
        { }

        bool operator==(Semantic const& rhs) const override;
        std::list<cs::Entry> into_entries() const override;
        std::list<cs::LinkEntry> into_link_entries() const;
        std::ostream& operator<<(std::ostream& os) const override;

    private:
        fs::path working_dir;
        fs::path linker;
        std::list<std::string> flags;
        std::list<fs::path> input_files;
        std::optional<fs::path> output;
    };

    class Ar : public CompilerCall {
    public:
        Ar(fs::path working_dir,
           fs::path ar_tool,
           std::string operation,
           std::list<std::string> flags,
           std::list<fs::path> input_files,
           std::optional<fs::path> output = std::nullopt)
            : working_dir(std::move(working_dir))
            , ar_tool(std::move(ar_tool))
            , operation(std::move(operation))
            , flags(std::move(flags))
            , input_files(std::move(input_files))
            , output(std::move(output))
        { }

        bool operator==(Semantic const& rhs) const override;
        std::list<cs::Entry> into_entries() const override;
        std::list<cs::ArEntry> into_ar_entries() const;
        std::ostream& operator<<(std::ostream& os) const override;

    private:
        fs::path working_dir;
        fs::path ar_tool;
        std::string operation;  // Keep this for internal use
        std::list<std::string> flags;  // This will include the operation
        std::list<fs::path> input_files;
        std::optional<fs::path> output;
    };
}
