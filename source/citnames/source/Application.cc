/*  Copyright (C) 2012-2021 by László Nagy
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
#include "Application.h"
#include "citnames/Flags.h"
#include "Configuration.h"
#include "Output.h"
#include "semantic/Build.h"
#include "semantic/Tool.h"
#include "collect/db/EventsDatabaseReader.h"

#include <filesystem>

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

namespace fs = std::filesystem;
namespace db = ic::collect::db;

namespace {

    bool is_exists(const fs::path &path) {
        std::error_code error_code;
        return fs::exists(path, error_code);
    }

    rust::Result<cs::Arguments> into_arguments(const flags::Arguments &args) {
        auto input = args.as_string(cs::INPUT);
        auto output = args.as_string(cs::OUTPUT);
        auto append = args.as_bool(cs::APPEND)
                .unwrap_or(false);

        return rust::merge(input, output)
                .map<cs::Arguments>([&append](auto tuple) {
                    const auto&[input, output] = tuple;
                    return cs::Arguments{
                            fs::path(input),
                            fs::path(output),
                            append,
                    };
                })
                .and_then<cs::Arguments>([](auto arguments) -> rust::Result<cs::Arguments> {
                    // validate
                    if (!is_exists(arguments.input)) {
                        return rust::Err(std::runtime_error(
                                fmt::format("Missing input file: {}", arguments.input)));
                    }
                    return rust::Ok(cs::Arguments{
                            arguments.input,
                            arguments.output,
                            (arguments.append && is_exists(arguments.output)),
                    });
                });
    }

    std::list<fs::path> compilers(const sys::env::Vars &environment) {
        std::list<fs::path> result;
        if (auto it = environment.find("CC"); it != environment.end()) {
            result.emplace_back(it->second);
        }
        if (auto it = environment.find("CXX"); it != environment.end()) {
            result.emplace_back(it->second);
        }
        if (auto it = environment.find("FC"); it != environment.end()) {
            result.emplace_back(it->second);
        }
        return result;
    }

    rust::Result<cs::Configuration>
    into_configuration(const flags::Arguments &args, const sys::env::Vars &environment) {
        auto config_arg = args.as_string(cs::CONFIG);
        auto config = config_arg.is_ok()
                      ? config_arg
                              .and_then<cs::Configuration>([](auto candidate) {
                                  return cs::ConfigurationSerializer().from_json(fs::path(candidate));
                              })
                      : rust::Ok(cs::Configuration());

        return config.map<cs::Configuration>([&args](auto config) {
                    // command line arguments overrides the default values or the configuration content.
                    args.as_bool(cs::RUN_CHECKS)
                            .on_success([&config](auto run) {
                                config.output.content.include_only_existing_source = run;
                            });

                    return config;
                })
                .map<cs::Configuration>([&environment](auto config) {
                    // recognize compilers from known environment variables.
                    for (const auto &compiler : compilers(environment)) {
                        auto wrapped = cs::CompilerWrapper{compiler, {}};
                        config.compilation.compilers_to_recognize.emplace_back(wrapped);
                    }
                    return config;
                })
                .on_success([](const auto &config) {
                    spdlog::debug("Configuration: {}", config);
                });
    }

    cs::Entries transform(cs::semantic::Build &build, const db::EventsDatabaseReader::Ptr& events) {
        cs::Entries results;
        for (db::EventsIterator it = events->events_begin(), end = events->events_end(); it != end; ++it) {
            (*it)
                    .and_then<cs::semantic::SemanticPtr>([&build](const auto &event) {
                        return build.recognize(*event);
                    })
                    .on_success([&results](const auto &semantic) {
                        auto candidate = dynamic_cast<const cs::semantic::CompilerCall *>(semantic.get());
                        if (candidate != nullptr) {
                            auto entries = candidate->into_entries();
                            std::copy(entries.begin(), entries.end(), std::back_inserter(results));
                        }
                    });
        }
        return results;
    }
}

namespace cs {

    rust::Result<int> Command::execute() const {
        cs::CompilationDatabase output(configuration_.output.format, configuration_.output.content);

        // get current compilations from the input.
        return db::EventsDatabaseReader::open(arguments_.input)
                .map<Entries>([this](const auto &commands) {
                    auto build = cs::semantic::Build(configuration_.compilation);
                    auto compilations = transform(build, commands);
                    // remove duplicates
                    return merge({}, compilations);
                })
                .and_then<Entries>([this, &output](const auto &compilations) {
                    // read back the current content and extend with the new elements.
                    spdlog::debug("compilation entries created. [size: {}]", compilations.size());
                    return (arguments_.append)
                           ? output.from_json(arguments_.output.c_str())
                                   .template map<Entries>([&compilations](auto old_entries) {
                                       spdlog::debug("compilation entries have read. [size: {}]", old_entries.size());
                                       return merge(compilations, old_entries);
                                   })
                           : rust::Result<Entries>(rust::Ok(compilations));
                })
                .and_then<size_t>([this, &output](const auto &compilations) {
                    // write the entries into the output file.
                    spdlog::debug("compilation entries to output. [size: {}]", compilations.size());
                    return output.to_json(arguments_.output.c_str(), compilations);
                })
                .map<int>([](auto size) {
                    // just map to success exit code if it was successful.
                    spdlog::debug("compilation entries written. [size: {}]", size);
                    return EXIT_SUCCESS;
                });
    }

    Command::Command(Arguments arguments, cs::Configuration configuration) noexcept
            : ps::Command()
            , arguments_(std::move(arguments))
            , configuration_(std::move(configuration))
    { }

    Application::Application() noexcept
            : ps::ApplicationFromArgs(ps::ApplicationLogConfig("citnames", "cs"))
    { }

    rust::Result<flags::Arguments> Application::parse(int argc, const char **argv) const {

        const flags::Parser parser(
                "citnames",
                VERSION,
                {
                        {cs::INPUT,      {1, false, "path of the input file",                    {"commands.sqlite3"},         std::nullopt}},
                        {cs::OUTPUT,     {1, false, "path of the result file",                   {"compile_commands.json"}, std::nullopt}},
                        {cs::CONFIG,     {1, false, "path of the config file",                   std::nullopt,              std::nullopt}},
                        {cs::APPEND,     {0, false, "append to output, instead of overwrite it", std::nullopt,              std::nullopt}},
                        {cs::RUN_CHECKS, {0, false, "can run checks on the current host",        std::nullopt,              std::nullopt}}
                });
        return parser.parse_or_exit(argc, const_cast<const char **>(argv));
    }

    rust::Result<ps::CommandPtr> Application::command(const flags::Arguments &args, const char **envp) const {
        auto environment = sys::env::from(const_cast<const char **>(envp));

        auto arguments = into_arguments(args);
        auto configuration = into_configuration(args, environment);

        return rust::merge(arguments, configuration)
                .map<ps::CommandPtr>([](auto tuples) {
                    const auto&[arguments, configuration] = tuples;
                    // read the configuration
                    return std::make_unique<Command>(arguments, configuration);
                });
    }
}
