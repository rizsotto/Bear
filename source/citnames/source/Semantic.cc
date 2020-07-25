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

    enum class Relevance {
        NOOP, // --help, --version
        ASK_PREPROCESSING_ONLY, // -E
        ASK_COMPILING_ONLY, // -c
        ASK_ASSEMBLY_ONLY, // -S
        CLANG_INTERNAL, // -cc1, -cc1as
        NOT_APPLICABLE,
    };

    bool shall_record(Relevance relevance) {
        return (relevance != Relevance::NOOP)
            && (relevance != Relevance::ASK_PREPROCESSING_ONLY)
            && (relevance != Relevance::CLANG_INTERNAL);
    }

    enum class CompilerFlagType {
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
        virtual ~CompilerFlag() noexcept = default;
        [[nodiscard]] virtual std::list<std::string> to_arguments() const = 0;
        [[nodiscard]] virtual bool is_source() { return false; }
        [[nodiscard]] virtual bool is_output() { return false; }
    };

    struct CompilerFlagWithOutput : public CompilerFlag {
        std::string path;

        [[nodiscard]] std::list<std::string> to_arguments() const override {
            return { "-o", path };
        }

        [[nodiscard]] bool is_output() override {
            return true;
        }
    };

    struct CompilerFlagWithSource : public CompilerFlag {
        std::string path;

        [[nodiscard]] std::list<std::string> to_arguments() const override {
            return { path };
        }

        [[nodiscard]] bool is_source() override {
            return true;
        }
    };

    struct CompilerFlagContainer : public CompilerFlag {
        std::list<std::string> arguments;

        [[nodiscard]] std::list<std::string> to_arguments() const override {
            return arguments;
        }
    };

    using CompilerFlagPtr = std::shared_ptr<CompilerFlag>;
    using CompilerFlags = std::list<CompilerFlagPtr>;

    struct CompilerCall : public cs::Semantic {

        [[nodiscard]] std::list<cs::output::Entry> into_compilation(const cs::cfg::Content &content) const override
        {
            std::list<cs::output::Entry> result;
            if (!shall_record(flags)) {
                return result;
            }

            auto output = to_output_file(flags);
            auto sources = to_source_files(flags);
            if (sources.empty()) {
                return result;
            }
            for (const auto& source : sources) {
                if (shall_include(source, content)) {
                    auto arguments = to_arguments(flags);
                    arguments.push_front(program);
                    cs::output::Entry entry = { source, directory, output, arguments };
                    result.emplace_back(std::move(entry));
                }
            }
            return result;
        }

    private:
        [[nodiscard]] bool shall_include(const std::string& source, const cs::cfg::Content &content) const
        {
            // source file might have relative dir, while the content filter is absolute!!!

            // TODO: use Content to filter/modify the entry
            return true;
        }

        [[nodiscard]] static bool shall_record(const CompilerFlags& flags)
        {
            // TODO
            return true;
        }

        [[nodiscard]] static std::list<std::string> to_source_files(const CompilerFlags& flags)
        {
            std::list<std::string> result;
            for (const auto& flag : flags) {
                if (flag->is_source()) {
                    const auto arguments = flag->to_arguments();
                    std::copy(arguments.begin(), arguments.end(), std::back_inserter(result));
                }
            }
            return result;
        }

        [[nodiscard]] static std::optional<std::string> to_output_file(const CompilerFlags& flags)
        {
            for (const auto& flag : flags) {
                if (flag->is_output()) {
                    const auto arguments = flag->to_arguments();
                    return std::optional<std::string>(arguments.back());
                }
            }
            return std::optional<std::string>();
        }

        [[nodiscard]] static std::list<std::string> to_arguments(const CompilerFlags& flags)
        {
            std::list<std::string> result;
            for (const auto& flag : flags) {
                const auto arguments = flag->to_arguments();
                std::copy(arguments.begin(), arguments.end(), std::back_inserter(result));
            }
            return result;
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

        static CompilerFlags parse_flags(const std::list<std::string> &arguments,
                                  const std::string& working_directory,
                                  const std::map<std::string, std::string>& environment)
        {
            // TODO:
            return CompilerFlags();
        }

        static std::regex into_regex(const std::list<std::string>& patterns)
        {
            auto pattern = fmt::format("({})", fmt::join(patterns.begin(), patterns.end(), "|"));
            return std::regex(pattern);
        }

        [[nodiscard]] bool program_matches(const std::string& program) const {
            std::cmatch m;
            return std::regex_match(program.c_str(), m, regex);
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
