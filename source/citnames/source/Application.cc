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
#include "Config.h"
#include "CompilationDatabase.h"
#include "Semantic.h"

#include "libreport/Report.h"

#include <fmt/format.h>

namespace {

    rust::Result<cs::Arguments> into_arguments(const flags::Arguments& args)
    {
        auto input = args.as_string(cs::Application::INPUT);
        auto output = args.as_string(cs::Application::OUTPUT);
        auto append = args.as_bool(cs::Application::APPEND).unwrap_or(false);
        auto run_check = args.as_bool(cs::Application::RUN_CHECKS).unwrap_or(false);

        return rust::merge(input, output)
                .map<cs::Arguments>([&append, &run_check](auto tuple) {
                    const auto& [input, output] = tuple;
                    return cs::Arguments {
                        std::string(input),
                        std::string(output),
                        append,
                        run_check
                    };
                });
    }
}

namespace cs {

    struct Application::State {
        cs::Arguments arguments;
        cs::cfg::Configuration configuration;
        cs::Semantic semantic;
        cs::output::CompilationDatabase output;
    };

    rust::Result<Application> Application::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        return into_arguments(args)
                .and_then<Application::State*>([&ctx](auto arguments) {
                    // modify the arguments till we have context for IO
                    arguments.append &= (ctx.is_exists(arguments.output) == 0);
                    if (ctx.is_exists(arguments.input) == 0) {
                        return rust::Result<Application::State*>(rust::Err(
                                std::runtime_error(fmt::format("Missing input file: {}", arguments.input))));
                    }
                    // read the configuration
                    auto configuration = cfg::default_value();
                    auto semantic = (arguments.run_check)
                            ? Semantic::from(configuration, ctx)
                            : Semantic::from(configuration);
                    cs::output::CompilationDatabase output;
                    return semantic.template map<Application::State*>([&arguments, &configuration, &output](auto semantic) {
                        return new Application::State { arguments, configuration, semantic, output };
                    });
                })
                .map<Application>([](auto impl) {
                    return Application { impl };
                });
    }

    rust::Result<int> Application::operator()() const
    {
        // get current compilations from the input.
        return report::from_json(impl_->arguments.input.c_str())
            .map<output::Entries>([this](auto commands) {
                return impl_->semantic.run(commands);
            })
            // read back the current content and extend with the new elements.
            .and_then<output::Entries>([this](auto compilations) {
                return (impl_->arguments.append)
                    ? impl_->output.from_json(impl_->arguments.output.c_str())
                            .template map<output::Entries>([&compilations](auto old_entries) {
                                return output::merge(old_entries, compilations);
                            })
                    : rust::Result<output::Entries>(rust::Ok(compilations));
            })
            // write the entries into the output file.
            .and_then<int>([this](auto compilations) {
                return impl_->output.to_json(impl_->arguments.output.c_str(), compilations, impl_->configuration.format);
            })
            // just map to success exit code if it was successful.
            .map<int>([](auto ignore) {
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
