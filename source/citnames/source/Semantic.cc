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

#include "Semantic.h"
#include "libsys/Path.h"

#include <regex>
#include <vector>

#include <fmt/format.h>

namespace {

    using path_fixer = std::function<std::string(const std::string&)>;

    path_fixer make_path_fixer(const std::string& working_directory, const std::optional<std::string>& requested)
    {
        // TODO: implement it
        return [](const auto& path) { return path; };
    }

    enum CompilerFlagType {
        // https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html
        KIND_OF_OUTPUT,
        LANGUAGE_DIALECT,
        DIAGNOSTIC,
        WARNING,
        ANALYZER,
        OPTIMIZATION,
        INSTRUMENTATION,
        PREPROCESSOR,
        ASSEMBLER,
        LINKER,
        DIRECTORY_SEARCH,
        CODE_GENERATION,
        DEVELOPER,
        MACHINE_DEPENDENT,
        // for other types
        UNKNOWN,
    };

    struct CompilerFlag {
        std::list<std::string> values;
        CompilerFlagType type;
    };

    using CompilerFlags = std::list<CompilerFlag>;

    CompilerFlags parse_flags(const std::list<std::string> &arguments,
                              const std::string& working_directory,
                              const std::map<std::string, std::string> environment)
    {
        // TODO:
        return CompilerFlags();
    }

    std::list<std::string> to_source_files(path_fixer path, const CompilerFlags& flags)
    {
        // TODO:
        return std::list<std::string>();
    }

    std::optional<std::string> to_output_file(path_fixer path, const CompilerFlags& flags)
    {
        // TODO:
        return std::optional<std::string>();
    }

    std::list<std::string> to_arguments(path_fixer path, const CompilerFlags& flags)
    {
        // TODO:
        return std::list<std::string>();
    }

    struct CompilerCall : public cs::Semantic {

        [[nodiscard]] std::list<cs::output::Entry> into_compilation(const cs::cfg::Content &content) const override {
            auto relative_to = make_path_fixer(directory, content.relative_to);

            auto sources = to_source_files(relative_to, flags);
            if (sources.empty()) {
                return std::list<cs::output::Entry>();
            }
            std::list<cs::output::Entry> result;
            auto output = to_output_file(relative_to, flags);
            for (const auto& source : sources) {
                if (shall_include(source, content)) {
                    auto arguments = to_arguments(relative_to, flags);
                    arguments.push_front(program);
                    cs::output::Entry entry = { source, relative_to(directory), output, arguments };
                    result.emplace_back(std::move(entry));
                }
            }
            return result;
        }

        [[nodiscard]] bool shall_include(const std::string& source, const cs::cfg::Content &content) const {
            // source file might have relative dir, while the content filter is absolute!!!

            // TODO: use Content to filter/modify the entry
            return true;
        }

    public:
        // presume every path is absolute.
        std::string directory;
        std::string program;
        // these can be represented better with a single attribute?
        CompilerFlags flags;
    };

    // Responsible to recognize the compiler by its name.
    struct Compiler : public cs::Tool {

        explicit Compiler(const std::list<std::string>& compilers)
                : regex(into_regex(compilers))
        { }

        [[nodiscard]] cs::SemanticPtr is_a(const report::Execution::Command &command) const override {

            if (program_matches(command.program)) {

                auto result = std::make_shared<CompilerCall>();

                result->directory = command.working_dir;
                result->program = command.program;
                result->flags = parse_flags(command.arguments, command.working_dir, command.environment);

                return result;
            }
            return std::shared_ptr<CompilerCall>();
        }

    private:
        static std::regex into_regex(const std::list<std::string>& patterns)
        {
            auto pattern = fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|"));
            return std::regex(pattern);
        }

        [[nodiscard]] bool program_matches(const std::string& program) const {
            std::cmatch m;
            auto basename = sys::path::basename(program);
            return std::regex_match(basename.c_str(), m, regex);
        }

    private:
        std::regex regex;
    };
}

namespace cs {

    Expert::Expert(const cfg::Value& config, Tools && tools) noexcept
            : config_(config)
            , tools_(tools)
    { }

    rust::Result<Expert> Expert::from(const cfg::Value& cfg)
    {
        // TODO: consider environment variables as hint for compiler
        //       CC, CXX and FC (maybe CPP too?)
        // TODO: use the other filters from cfg::Compilation
        try {
            auto compilers = cfg.compilation.compilers;
            Tools tools = {
                    // TODO: create new types
                    std::make_shared<Compiler>(compilers.mpi),
                    std::make_shared<Compiler>(compilers.cuda),
                    std::make_shared<Compiler>(compilers.distcc),
                    std::make_shared<Compiler>(compilers.ccache),
                    std::make_shared<Compiler>(compilers.cc),
                    std::make_shared<Compiler>(compilers.cxx),
                    std::make_shared<Compiler>(compilers.fortran),
            };
            return rust::Ok(Expert(cfg, std::move(tools)));
        } catch (const std::runtime_error &error) {
            return rust::Err(error);
        }
    }

    rust::Result<Expert> Expert::from(const cfg::Value& cfg, const sys::Context& ctx)
    {
        // TODO: add the capability to check things on the host
        return from(cfg);
    }

    output::Entries Expert::transform(const report::Report& report) const
    {
        output::Entries result;
        for (const auto& execution : report.executions) {
            if (auto semantic = recognize(execution.command); semantic) {
                auto entries = semantic->into_compilation(config_.content);
                std::copy(entries.begin(), entries.end(), std::back_inserter(result));
            }
        }
        return result;
    }

    SemanticPtr Expert::recognize(const report::Execution::Command& command) const
    {
        for (const auto& tool : tools_) {
            if (auto semantic = tool->is_a(command); semantic) {
                return semantic;
            }
        }
        return std::shared_ptr<Semantic>();
    }
}
