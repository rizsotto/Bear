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

    bool rename_file(const fs::path &from, const fs::path &to) {
        std::error_code error_code;
        fs::rename(from, to, error_code);
        return error_code.value() == 0;
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

    size_t transform_links(cs::semantic::Build &build, const db::EventsDatabaseReader::Ptr& events, std::list<cs::LinkEntry> &output) {
        for (const auto &event : *events) {
            const auto entries = build.recognize(event)
                    .map<std::list<cs::LinkEntry>>([](const auto &semantic) -> std::list<cs::LinkEntry> {
                        const auto candidate = dynamic_cast<const cs::semantic::Link *>(semantic.get());
                        return (candidate != nullptr) ? candidate->into_link_entries() : std::list<cs::LinkEntry>();
                    })
                    .unwrap_or({});
            std::copy(entries.begin(), entries.end(), std::back_inserter(output));
        }
        return output.size();
    }

    size_t transform_ar(cs::semantic::Build &build, const db::EventsDatabaseReader::Ptr& events, std::list<cs::ArEntry> &output) {
        for (const auto &event : *events) {
            const auto entries = build.recognize(event)
                    .map<std::list<cs::ArEntry>>([](const auto &semantic) -> std::list<cs::ArEntry> {
                        const auto candidate = dynamic_cast<const cs::semantic::Ar *>(semantic.get());
                        return (candidate != nullptr) ? candidate->into_ar_entries() : std::list<cs::ArEntry>();
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
        std::list<cs::LinkEntry> link_entries;
        std::list<cs::ArEntry> ar_entries;

        // get current compilations from the input.
        auto compile_result = db::EventsDatabaseReader::from(arguments_.input)
                .map<size_t>([this, &entries](const auto &commands) {
                    cs::semantic::Build build(configuration_.compilation);
                    return transform(build, commands, entries);
                })
                .and_then<size_t>([this, &output, &entries](auto new_entries_count) {
                    spdlog::debug("compilation entries created. [size: {}]", new_entries_count);
                    // read back the current content and extend with the new elements.
                    return (arguments_.append)
                        ? output.from_json(arguments_.output, entries)
                                .template map<size_t>([&new_entries_count](auto old_entries_count) {
                                    spdlog::debug("compilation entries have read. [size: {}]", old_entries_count);
                                    return new_entries_count + old_entries_count;
                                })
                        : rust::Result<size_t>(rust::Ok(new_entries_count));
                })
                .and_then<size_t>([this, &output, &entries](auto size) {
                    // write the entries into the output file.
                    spdlog::debug("compilation entries to output. [size: {}]", size);

                    const fs::path temporary_file(arguments_.output.string() + ".tmp");
                    auto result = output.to_json(temporary_file, entries);
                    return rename_file(temporary_file, arguments_.output)
                        ? result
                        : rust::Err(std::runtime_error(fmt::format("Failed to rename file: {}", arguments_.output)));
                });

        auto link_result = rust::Result<size_t>(rust::Ok<size_t>(0));
        if (!configuration_.output.link_commands_output.empty()) {
            link_result = db::EventsDatabaseReader::from(arguments_.input)
                .map<size_t>([this, &link_entries](const auto &commands) {
                    cs::semantic::Build build(configuration_.compilation);
                    return transform_links(build, commands, link_entries);
                })
                .and_then<size_t>([this, &output, &link_entries](auto new_entries_count) {
                    spdlog::debug("link entries created. [size: {}]", new_entries_count);
                    return (arguments_.append)
                        ? output.from_link_json(arguments_.output, link_entries)
                                .template map<size_t>([&new_entries_count](auto old_entries_count) {
                                    spdlog::debug("link entries have read. [size: {}]", old_entries_count);
                                    return new_entries_count + old_entries_count;
                                })
                        : rust::Result<size_t>(rust::Ok(new_entries_count));
                })
                .and_then<size_t>([this, &output, &link_entries](auto size) {
                    // write the link entries into a separate output file if configured
                    spdlog::debug("link entries to output. [size: {}]", size);

                    const fs::path link_temp_file(configuration_.output.link_commands_output.string() + ".tmp");
                    auto link_write_result = output.to_link_json(link_temp_file, link_entries);
                    return rename_file(link_temp_file, configuration_.output.link_commands_output)
                        ? link_write_result
                        : rust::Err(std::runtime_error(fmt::format("Failed to rename file: {}", configuration_.output.link_commands_output)));
                });
        }

        auto ar_result = rust::Result<size_t>(rust::Ok<size_t>(0));
        if (!configuration_.output.ar_commands_output.empty()) {
            ar_result = db::EventsDatabaseReader::from(arguments_.input)
                .map<size_t>([this, &ar_entries](const auto &commands) {
                    cs::semantic::Build build(configuration_.compilation);
                    return transform_ar(build, commands, ar_entries);
                })
                .and_then<size_t>([this, &output, &ar_entries](auto new_entries_count) {
                    spdlog::debug("ar entries created. [size: {}]", new_entries_count);
                    return (arguments_.append)
                        ? output.from_ar_json(arguments_.output, ar_entries)
                                .template map<size_t>([&new_entries_count](auto old_entries_count) {
                                    spdlog::debug("ar entries have read. [size: {}]", old_entries_count);
                                    return new_entries_count + old_entries_count;
                                })
                        : rust::Result<size_t>(rust::Ok(new_entries_count));
                })
                .and_then<size_t>([this, &output, &ar_entries](auto size) {
                    // write the ar entries into a separate output file if configured
                    spdlog::debug("ar entries to output. [size: {}]", size);

                    const fs::path ar_temp_file(configuration_.output.ar_commands_output.string() + ".tmp");
                    auto ar_write_result = output.to_ar_json(ar_temp_file, ar_entries);
                    return rename_file(ar_temp_file, configuration_.output.ar_commands_output)
                        ? ar_write_result
                        : rust::Err(std::runtime_error(fmt::format("Failed to rename file: {}", configuration_.output.ar_commands_output)));
                });
        }

        return rust::merge(compile_result, rust::merge(link_result, ar_result))
            .map<int>([](const auto &sizes) -> int {
                const auto&[compile_size, link_and_ar_sizes] = sizes;
                const auto&[link_size, ar_size] = link_and_ar_sizes;
                spdlog::debug("compilation entries written. [size: {}]", compile_size);
                return compile_size + link_size + ar_size;
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
