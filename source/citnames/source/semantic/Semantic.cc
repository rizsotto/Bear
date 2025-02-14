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

#include "config.h"
#include "semantic/Semantic.h"

#include <fmt/format.h>
#include <fmt/ranges.h>
#ifdef HAVE_FMT_STD_H
#include <fmt/std.h>
#else
namespace fmt {
    template <>
    struct formatter<fs::path> : formatter<std::string> {};
}
#endif

namespace cs::semantic {

    bool QueryCompiler::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto *const ptr = dynamic_cast<QueryCompiler const*>(&rhs); ptr != nullptr) {
            return true;
        }
        return false;
    }

    std::ostream &QueryCompiler::operator<<(std::ostream &os) const {
        os << "Query";
        return os;
    }

    std::list<cs::Entry> QueryCompiler::into_entries() const {
        return {};
    }

    bool Preprocess::operator==(const Semantic &) const {
        return false;
    }

    std::ostream &Preprocess::operator<<(std::ostream &os) const {
        os << "Preprocess";
        return os;
    }

    std::list<cs::Entry> Preprocess::into_entries() const {
        return {};
    }

    Compile::Compile(fs::path working_dir,
                     fs::path compiler,
                     std::list<std::string> flags,
                     std::vector<fs::path> sources,
                     std::optional<fs::path> output)
            : working_dir(std::move(working_dir))
            , compiler(std::move(compiler))
            , flags(std::move(flags))
            , sources(std::move(sources))
            , output(std::move(output))
    { }

    bool Compile::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto *const ptr = dynamic_cast<Compile const*>(&rhs); ptr != nullptr) {
            return (working_dir == ptr->working_dir)
                && (compiler == ptr->compiler)
                && (output == ptr->output)
                && (sources == ptr->sources)
                && (flags == ptr->flags);
        }
        return false;
    }

    std::ostream &Compile::operator<<(std::ostream &os) const {
        os  << "Compile { working_dir: " << working_dir
            << ", compiler: " << compiler
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << ", sources: " << fmt::format("[{}]", fmt::join(sources.begin(), sources.end(), ", "))
            << ", output: " << (output ? output.value().string() : "")
            << " }";
        return os;
    }

    std::list<cs::Entry> Compile::into_entries() const {
        const auto abspath = [this](const fs::path &path) -> fs::path {
            auto candidate = (path.is_absolute()) ? path : working_dir / path;
            // Create canonical path without checking of file existence.
            fs::path result;
            for (const auto& part : candidate) {
                if (part == ".")
                    continue;
                if (part == "..")
                    result = result.parent_path();
                else
                    result = result / part;
            }
            return result;
        };
        std::list<cs::Entry> results;
        for (const auto& source : sources) {
            cs::Entry result {
                abspath(source),
                working_dir,
                output ? std::optional(abspath(output.value())) : std::nullopt,
                { compiler.string() }
            };
            std::copy(flags.begin(), flags.end(), std::back_inserter(result.arguments));
            if (output) {
                result.arguments.emplace_back("-o");
                result.arguments.push_back(output.value().string());
            }
            result.arguments.push_back(source);
            results.emplace_back(std::move(result));
        }
        return results;
    }

    std::list<cs::Entry> Link::into_entries() const {
        return {};
    }

    std::list<cs::LinkEntry> Link::into_link_entries() const {
        const auto abspath = [this](const fs::path &path) -> fs::path {
            auto candidate = (path.is_absolute()) ? path : working_dir / path;
            // Create canonical path without checking of file existence.
            fs::path result;
            for (const auto& part : candidate) {
                if (part == ".")
                    continue;
                if (part == "..")
                    result = result.parent_path();
                else
                    result = result / part;
            }
            return result;
        };
        std::list<cs::LinkEntry> results;
        cs::LinkEntry result {
            working_dir,
            output ? std::optional(abspath(output.value())) : std::nullopt,
            { linker.string() },
            {}
        };
        std::copy(flags.begin(), flags.end(), std::back_inserter(result.arguments));
        if (output) {
            result.arguments.emplace_back("-o");
            result.arguments.push_back(output.value().string());
        }
        for (const auto& input : input_files) {
            result.input_files.push_back(abspath(input));
            result.arguments.push_back(input.string());
        }
        results.emplace_back(std::move(result));
        return results;
    }

    std::ostream &Link::operator<<(std::ostream &os) const {
        os  << "Link { working_dir: " << working_dir
            << ", linker: " << linker
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << ", input_files: " << fmt::format("[{}]", fmt::join(input_files.begin(), input_files.end(), ", "))
            << ", output: " << (output ? output.value().string() : "")
            << " }";
        return os;
    }

    bool Link::operator==(const Semantic &rhs) const {
        if (this == &rhs) {
            return true;
        }
        if (const auto *const ptr = dynamic_cast<const Link*>(&rhs)) {
            return (working_dir == ptr->working_dir)
                && (linker == ptr->linker)
                && (flags == ptr->flags)
                && (input_files == ptr->input_files)
                && (output == ptr->output);
        }
        return false;
    }

    bool Ar::operator==(const Semantic &rhs) const {
        if (this == &rhs) {
            return true;
        }
        if (const auto *const ptr = dynamic_cast<const Ar*>(&rhs)) {
            return (working_dir == ptr->working_dir)
                && (ar_tool == ptr->ar_tool)
                && (operation == ptr->operation)
                && (flags == ptr->flags)
                && (input_files == ptr->input_files)
                && (output == ptr->output);
        }
        return false;
    }

    std::list<cs::Entry> Ar::into_entries() const {
        return {};
    }

    std::list<cs::ArEntry> Ar::into_ar_entries() const {
        const auto abspath = [this](const fs::path &path) -> fs::path {
            auto candidate = (path.is_absolute()) ? path : working_dir / path;
            // Create canonical path without checking of file existence.
            fs::path result;
            for (const auto& part : candidate) {
                if (part == ".")
                    continue;
                if (part == "..")
                    result = result.parent_path();
                else
                    result = result / part;
            }
            return result;
        };
        std::list<cs::ArEntry> results;
        cs::ArEntry result {
            working_dir,
            output ? std::optional(abspath(output.value())) : std::nullopt,
            { ar_tool.string() },
            {},
            operation
        };

        // Build arguments list in correct order:
        // 1. Copy all flags (which include operation, modifiers)
        std::copy(flags.begin(), flags.end(), std::back_inserter(result.arguments));

        // 2. Add archive file
        if (output) {
            result.arguments.push_back(output.value().string());
        }
        
        // 3. Add input files and record them in input_files
        for (const auto& input : input_files) {
            result.input_files.push_back(abspath(input));
            result.arguments.push_back(input.string());
        }
        
        results.emplace_back(std::move(result));
        return results;
    }

    std::ostream &Ar::operator<<(std::ostream &os) const {
        os  << "Ar { working_dir: " << working_dir
            << ", ar_tool: " << ar_tool
            << ", operation: " << operation
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << ", input_files: " << fmt::format("[{}]", fmt::join(input_files.begin(), input_files.end(), ", "))
            << ", output: " << (output ? output.value().string() : "")
            << " }";
        return os;
    }
}
