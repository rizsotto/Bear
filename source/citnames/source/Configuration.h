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

#include <libresult/Result.h>

#include <filesystem>
#include <iosfwd>
#include <list>
#include <map>
#include <string>
#include <optional>
#include <utility>

namespace fs = std::filesystem;

namespace cs {

    // Controls the output format.
    //
    // The entries in the JSON compilation database can have different forms.
    // One format element is how the command is represented: it can be an array
    // of strings or a single string (shell escaping to protect white spaces).
    // Another format element is if the output field is emitted or not.
    struct Format {
        bool command_as_array;
        bool drop_output_field;

        constexpr Format() noexcept
                : command_as_array(true)
                , drop_output_field(false)
        { }

        constexpr Format(bool _command_as_array, bool _drop_output_field) noexcept
                : command_as_array(_command_as_array)
                , drop_output_field(_drop_output_field)
        { }
    };

    // Controls the content of the output.
    //
    // This will act as a filter on the output elements.
    // These attributes can be read from the configuration file, and can be
    // overridden by command line arguments.
    struct Content {
        bool include_only_existing_source;
        std::list<fs::path> paths_to_include;
        std::list<fs::path> paths_to_exclude;

        Content() noexcept
                : include_only_existing_source(false)
                , paths_to_include()
                , paths_to_exclude()
        { }

        Content(bool _include_only_existing_source,
                std::list<fs::path> _paths_to_include,
                std::list<fs::path> _paths_to_exclude) noexcept
                : include_only_existing_source(_include_only_existing_source)
                , paths_to_include(std::move(_paths_to_include))
                , paths_to_exclude(std::move(_paths_to_exclude))
        { }
    };

    // Groups together the output related configurations.
    struct Output {
        Format format;
        Content content;
    };

    // Represents a compiler wrapper that the tool will recognize.
    //
    // When executable name matches it tries to parse the flags as it would
    // be a known compiler, and append the additional flags to the output
    // entry if the compiler is recognized.
    struct CompilerWrapper {
        fs::path executable;
        std::list<std::string> additional_flags;
    };

    // Represents compiler related configuration.
    struct Compilation {
        std::list<CompilerWrapper> compilers_to_recognize;
        std::list<fs::path> compilers_to_exclude;
        std::list<std::string> flags_to_remove;
    };

    // Represents the application configuration.
    struct Configuration {
        Output output;
        Compilation compilation;
    };

    // Convenient methods for these types.
    std::ostream& operator<<(std::ostream&, const Format&);
    std::ostream& operator<<(std::ostream&, const Content&);
    std::ostream& operator<<(std::ostream&, const Output&);
    std::ostream& operator<<(std::ostream&, const CompilerWrapper&);
    std::ostream& operator<<(std::ostream&, const Compilation&);
    std::ostream& operator<<(std::ostream&, const Configuration&);

    // Utility class to persists configuration in JSON.
    struct ConfigurationSerializer {
        virtual ~ConfigurationSerializer() noexcept = default;

        // Serialization methods with error mapping.
        [[nodiscard]] virtual rust::Result<size_t> to_json(const fs::path &, const Configuration &rhs) const;
        [[nodiscard]] virtual rust::Result<size_t> to_json(std::ostream &ostream, const Configuration &rhs) const;

        [[nodiscard]] virtual rust::Result<Configuration> from_json(const fs::path &) const;
        [[nodiscard]] virtual rust::Result<Configuration> from_json(std::istream &istream) const;
    };
}
