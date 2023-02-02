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

#include "config.h"
#include "Citnames.h"
#include "Configuration.h"
#include "Output.h"
#include "semantic/Build.h"
#include "semantic/Tool.h"
#include "collect/db/EventsDatabaseReader.h"
#include "libsys/Path.h"

#include <filesystem>

#ifdef HAVE_FMT_STD_H
#include <fmt/std.h>
#endif
#include <fmt/ostream.h>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;
namespace db = ic::collect::db;

#ifdef FMT_NEEDS_OSTREAM_FORMATTER
template <> struct fmt::formatter<cs::Configuration> : ostream_formatter {};
#endif

namespace {

    std::list<fs::path> to_abspath(const std::list<fs::path> &paths, const fs::path &root) {
        std::list<fs::path> results;
        for (const auto &path : paths) {
            auto result = path.is_absolute() ? path : root / path;
            results.emplace_back(result);
        }
        return results;
    }

    cs::Content update_content(cs::Content content, bool run_checks) {
        if (run_checks) {
            auto cwd = sys::path::get_cwd();
            if (cwd.is_ok()) {
                const fs::path& root = cwd.unwrap();
                return cs::Content {
                        run_checks,
                        content.duplicate_filter_fields,
                        to_abspath(content.paths_to_include, root),
                        to_abspath(content.paths_to_exclude, root)
                };
            } else {
                spdlog::warn("Update configuration failed: {}", cwd.unwrap_err().what());
            }
        }
        return content;
    }

    std::list<cs::CompilerWrapper> update_compilers_to_recognize(
            std::list<cs::CompilerWrapper> wrappers,
            std::list<fs::path> compilers)
    {
        for (auto && compiler : compilers) {
            const bool already_in_wrappers =
                    std::any_of(wrappers.begin(), wrappers.end(),
                                [&compiler](auto wrapper) { return wrapper.executable == compiler; });
            if (!already_in_wrappers) {
                wrappers.emplace_back(cs::CompilerWrapper {
                    compiler,
                    {},
                    {}
                });
            }
        }
        return wrappers;
    }

    bool is_exists(const fs::path &path) {
        std::error_code error_code;
        return fs::exists(path, error_code);
    }

    rust::Result<cs::Arguments> into_arguments(const flags::Arguments &args) {
        auto input = args.as_string(cmd::citnames::FLAG_INPUT);
        auto output = args.as_string(cmd::citnames::FLAG_OUTPUT);
        auto append = args.as_bool(cmd::citnames::FLAG_APPEND)
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
        auto config_arg = args.as_string(cmd::citnames::FLAG_CONFIG);
        auto config = config_arg.is_ok()
                      ? config_arg
                              .and_then<cs::Configuration>([](auto candidate) {
                                  return cs::ConfigurationSerializer().from_json(fs::path(candidate));
                              })
                      : rust::Ok(cs::Configuration());

        return config
                .map<cs::Configuration>([&args](auto config) {
                    // command line arguments overrides the default values or the configuration content.
                    const auto run_checks = args
                            .as_bool(cmd::citnames::FLAG_RUN_CHECKS)
                            .unwrap_or(config.output.content.include_only_existing_source);
                    // update the content filter parameters according to the run_check outcome.
                    config.output.content = update_content(config.output.content, run_checks);
                    return config;
                })
                .map<cs::Configuration>([&environment](auto config) {
                    // recognize compilers from known environment variables.
                    const auto env_compilers = compilers(environment);
                    config.compilation.compilers_to_recognize =
                            update_compilers_to_recognize(config.compilation.compilers_to_recognize, env_compilers);
                    return config;
                })
                .on_success([](const auto &config) {
                    spdlog::debug("Configuration: {}", config);
                });
    }

    size_t transform(cs::semantic::Build &build, const db::EventsDatabaseReader::Ptr& events, std::list<cs::Entry> &output) {
        for (const auto &event : *events) {
            const auto entries = build.recognize(event)
                    .map<std::list<cs::Entry>>([](const auto &semantic) -> std::list<cs::Entry> {
                        const auto candidate = dynamic_cast<const cs::semantic::CompilerCall *>(semantic.get());
                        return (candidate != nullptr) ? candidate->into_entries() : std::list<cs::Entry>();
                    })
                    .unwrap_or({});
            std::copy(entries.begin(), entries.end(), std::back_inserter(output));
        }
        return output.size();
    }
}

namespace cs {

    rust::Result<int> Command::execute() const {
        cs::CompilationDatabase output(configuration_.output.format, configuration_.output.content);
        std::list<cs::Entry> entries;

        // get current compilations from the input.
        return db::EventsDatabaseReader::from(arguments_.input)
                .map<size_t>([this, &entries](const auto &commands) {
                    cs::semantic::Build build(configuration_.compilation);
                    return transform(build, commands, entries);
                })
                .and_then<size_t>([this, &output, &entries](auto new_entries_count) {
                    spdlog::debug("compilation entries created. [size: {}]", new_entries_count);
                    // read back the current content and extend with the new elements.
                    return (arguments_.append)
                        ? output.from_json(arguments_.output.c_str(), entries)
                                .template map<size_t>([&new_entries_count](auto old_entries_count) {
                                    spdlog::debug("compilation entries have read. [size: {}]", old_entries_count);
                                    return new_entries_count + old_entries_count;
                                })
                        : rust::Result<size_t>(rust::Ok(new_entries_count));
                })
                .and_then<size_t>([this, &output, &entries](const size_t & size) {
                    // write the entries into the output file.
                    spdlog::debug("compilation entries to output. [size: {}]", size);
                    return output.to_json(arguments_.output.c_str(), entries);
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

    Citnames::Citnames(const ps::ApplicationLogConfig& log_config) noexcept
            : ps::SubcommandFromArgs("citnames", log_config)
    { }

    rust::Result<ps::CommandPtr> Citnames::command(const flags::Arguments &args, const char **envp) const {
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
