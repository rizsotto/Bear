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

#include "Application.h"
#include "Configuration.h"
#include "Output.h"
#include "semantic/Tool.h"

#include "libreport/Report.h"

#include <filesystem>

#include <fmt/format.h>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace {

    bool is_exists(const fs::path& path)
    {
        std::error_code error_code;
        return fs::exists(path, error_code);
    }

    std::list<fs::path> to_path_list(const std::vector<std::string_view>& strings)
    {
        // best effort, try to make these string as path (absolute or relative).
        std::error_code error_code;
        auto cwd = fs::current_path(error_code);
        if (error_code) {
            spdlog::info("Getting current directory failed. (ignored)");
            return std::list<fs::path>(strings.begin(), strings.end());
        } else {
            std::list<fs::path> result;
            for (auto string : strings) {
                auto path = fs::path(string);
                if (path.is_absolute()) {
                    result.emplace_back(path);
                } else {
                    result.emplace_back(cwd / path);
                }
            }
            return result;
        }
    }

    struct Arguments {
        fs::path input;
        fs::path output;
        bool append;
    };

    rust::Result<Arguments> into_arguments(const flags::Arguments& args)
    {
        auto input = args.as_string(cs::Application::INPUT);
        auto output = args.as_string(cs::Application::OUTPUT);
        auto append = args.as_bool(cs::Application::APPEND)
                .unwrap_or(false);

        return rust::merge(input, output)
                .map<Arguments>([&append](auto tuple) {
                    const auto& [input, output] = tuple;
                    return Arguments {
                            fs::path(input),
                            fs::path(output),
                            append,
                    };
                })
                // validate
                .and_then<Arguments>([](auto arguments) -> rust::Result<Arguments> {
                    if (!is_exists(arguments.input)) {
                        return rust::Err(std::runtime_error(
                                fmt::format("Missing input file: {}", arguments.input)));
                    }
                    return rust::Ok(Arguments {
                            arguments.input,
                            arguments.output,
                            (arguments.append && is_exists(arguments.output)),
                    });
                });
    }

    std::list<fs::path> compilers(const sys::env::Vars& environment)
    {
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

    rust::Result<cs::Configuration> into_configuration(const flags::Arguments& args, const sys::env::Vars& environment)
    {
        auto config_arg = args.as_string(cs::Application::CONFIG);
        auto config = config_arg.is_ok()
                ? config_arg
                              .and_then<cs::Configuration>([](auto candidate) {
                                  return cs::ConfigurationSerializer().from_json(fs::path(candidate));
                              })
                : rust::Ok(cs::Configuration());

        // command line arguments overrides the default values or the configuration content.
        return config.map<cs::Configuration>([&args](auto config) {
            args.as_bool(cs::Application::RUN_CHECKS)
                    .on_success([&config](auto run) {
                        config.output.content.include_only_existing_source = run;
                    });
            args.as_string_list(cs::Application::INCLUDE)
                    .map<std::list<fs::path>>(&to_path_list)
                    .on_success([&config](auto includes) {
                        config.output.content.paths_to_include = includes;
                    });
            args.as_string_list(cs::Application::EXCLUDE)
                    .map<std::list<fs::path>>(&to_path_list)
                    .on_success([&config](auto excludes) {
                        config.output.content.paths_to_exclude = excludes;
                    });

            return config;
        })
        // recognize compilers from known environment variables.
        .map<cs::Configuration>([&environment](auto config) {
            for (const auto& compiler : compilers(environment)) {
                auto wrapped = cs::CompilerWrapper { compiler, {} };
                config.compilation.compilers_to_recognize.emplace_back(wrapped);
            }
            return config;
        })
        .on_success([](const auto& config) {
            spdlog::debug("Configuration: {}", config);
        });
    }
}

namespace cs {

    struct Application::State {
        Arguments arguments;
        report::ReportSerializer report_serializer;
        cs::semantic::Tools semantic;
        cs::CompilationDatabase output;
    };

    rust::Result<Application> Application::from(const flags::Arguments& args, sys::env::Vars&& environment)
    {
        auto arguments = into_arguments(args);
        auto configuration = into_configuration(args, environment);
        auto semantic = configuration.and_then<cs::semantic::Tools>([](auto config) {
            return semantic::Tools::from(config.compilation);
        });

        return rust::merge(arguments, configuration, semantic)
                .map<Application::State*>([](auto tuples) {
                    const auto& [arguments, configuration, semantic] = tuples;
                    // read the configuration
                    cs::CompilationDatabase output(configuration.output.format, configuration.output.content);
                    report::ReportSerializer report_serializer;
                    return new Application::State { arguments, report_serializer, semantic, output };
                })
                .map<Application>([](auto impl) {
                    spdlog::debug("application object initialized.");
                    return Application { impl };
                });
    }

    rust::Result<int> Application::operator()() const
    {
        // get current compilations from the input.
        return impl_->report_serializer.from_json(impl_->arguments.input)
            .map<Entries>([this](const auto& commands) {
                spdlog::debug("commands have read. [size: {}]", commands.executions.size());
                auto compilations = impl_->semantic.transform(commands);
                // remove duplicates
                return merge({}, compilations);
            })
            // read back the current content and extend with the new elements.
            .and_then<Entries>([this](const auto& compilations) {
                spdlog::debug("compilation entries created. [size: {}]", compilations.size());
                return (impl_->arguments.append)
                    ? impl_->output.from_json(impl_->arguments.output.c_str())
                            .template map<Entries>([&compilations](auto old_entries) {
                                spdlog::debug("compilation entries have read. [size: {}]", old_entries.size());
                                return merge(compilations, old_entries);
                            })
                    : rust::Result<Entries>(rust::Ok(compilations));
            })
            // write the entries into the output file.
            .and_then<size_t>([this](const auto& compilations) {
                spdlog::debug("compilation entries to output. [size: {}]", compilations.size());
                return impl_->output.to_json(impl_->arguments.output.c_str(), compilations);
            })
            // just map to success exit code if it was successful.
            .map<int>([](auto size) {
                spdlog::debug("compilation entries written. [size: {}]", size);
                return EXIT_SUCCESS;
            });
    }

    Application::Application(Application::State* const impl)
        : impl_(impl)
    {
    }

    Application::Application(Application&& rhs) noexcept
        : impl_(rhs.impl_)
    {
        rhs.impl_ = nullptr;
    }

    Application& Application::operator=(Application&& rhs) noexcept
    {
        if (&rhs != this) {
            delete impl_;
            impl_ = rhs.impl_;
        }
        return *this;
    }

    Application::~Application()
    {
        delete impl_;
        impl_ = nullptr;
    }
}
