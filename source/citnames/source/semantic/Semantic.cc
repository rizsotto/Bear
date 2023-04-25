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

#include "semantic/Semantic.h"

#include <fmt/format.h>

namespace {
    fs::path abspath(const fs::path &path, const fs::path &working_dir) {
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
    }

    std::list<fs::path> abspath(const std::list<fs::path> &files, const fs::path &working_dir) {
        std::list<fs::path> files_with_abspath;
        for (const auto& file : files) {
            files_with_abspath.push_back(abspath(file, working_dir));
        }
        return files_with_abspath;
    }
}

namespace fmt {

    template <>
    struct formatter<fs::path> : formatter<std::string> {};
}

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
                     std::list<fs::path> sources,
                     std::list<fs::path> dependencies,
                     std::optional<fs::path> output,
                     bool with_linking)
            : working_dir(std::move(working_dir))
            , compiler(std::move(compiler))
            , flags(std::move(flags))
            , sources(std::move(sources))
            , dependencies(std::move(dependencies))
            , output(std::move(output))
            , with_linking(with_linking)
    { }

    bool Compile::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto *const ptr = dynamic_cast<Compile const*>(&rhs); ptr != nullptr) {
            return (working_dir == ptr->working_dir)
                && (compiler == ptr->compiler)
                && (flags == ptr->flags)
                && (sources == ptr->sources)
                && (dependencies == ptr->dependencies)
                && (output == ptr->output)
                && (with_linking == ptr->with_linking);
        }
        return false;
    }

    std::ostream &Compile::operator<<(std::ostream &os) const {
        os  << "Compile { working_dir: " << working_dir
            << ", compiler: " << compiler
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << ", sources: " << fmt::format("[{}]", fmt::join(sources.begin(), sources.end(), ", "))
            << ", dependencies: " << fmt::format("[{}]", fmt::join(dependencies.begin(), dependencies.end(), ", "))
            << ", output: " << (output ? output.value().string() : "")
            << ", with_linking: " << with_linking
            << " }";
        return os;
    }

    std::list<cs::Entry> Compile::into_entries() const {
        const auto dependencies_abspath = abspath(dependencies, working_dir);
        std::list<cs::Entry> results;
        for (const auto& source : sources) {
            const fs::path real_output = (output && !with_linking)
                ? output.value()
                : fs::path(source.string() + ".o");

            cs::Entry result {
                abspath(source, working_dir),
                dependencies_abspath,
                working_dir,
                abspath(real_output, working_dir),
                { compiler.string() }
            };

            // flags contains everything except output and sources
            std::copy(flags.begin(), flags.end(), std::back_inserter(result.arguments));
            if (output) {
                result.arguments.emplace_back("-o");
                result.arguments.push_back(real_output);
            }
            result.arguments.push_back(source);

            results.emplace_back(std::move(result));
        }
        return results;
    }

    Link::Link(fs::path working_dir,
               fs::path compiler,
               std::list<std::string> flags,
               std::list<fs::path> files,
               std::optional<fs::path> output)
            : working_dir(std::move(working_dir))
            , compiler(std::move(compiler))
            , flags(std::move(flags))
            , files(std::move(files))
            , output(std::move(output))
    { }

    bool Link::operator==(const Semantic &rhs) const {
        if (this == &rhs)
            return true;

        if (const auto *const ptr = dynamic_cast<Link const*>(&rhs); ptr != nullptr) {
            return (working_dir == ptr->working_dir)
                && (compiler == ptr->compiler)
                && (flags == ptr->flags)
                && (files == ptr->files)
                && (output == ptr->output);
        }
        return false;
    }

    std::ostream &Link::operator<<(std::ostream &os) const {
        os  << "Link { working_dir: " << working_dir
            << ", compiler: " << compiler
            << ", flags: " << fmt::format("[{}]", fmt::join(flags.begin(), flags.end(), ", "))
            << ", files: " << fmt::format("[{}]", fmt::join(files.begin(), files.end(), ", "))
            << ", output: " << (output ? output.value().string() : "")
            << " }";
        return os;
    }

    std::list<cs::Entry> Link::into_entries() const {
        cs::Entry result {
            fs::path(),
            abspath(files, working_dir),
            working_dir,
            output ? std::optional(abspath(output.value(), working_dir)) : std::nullopt,
            { compiler.string() }
        };

        // flags contains everything except output
        std::copy(flags.begin(), flags.end(), std::back_inserter(result.arguments));
        if (output) {
            result.arguments.emplace_back("-o");
            result.arguments.push_back(output.value().string());
        }

        return std::list{result};
    }
}
