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

namespace {

    rust::Result<cs::Arguments> from(const flags::Arguments& args)
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
    };

    rust::Result<Application> Application::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        return ::from(args)
                .map<Application::State*>([](auto arguments) {
                    return new Application::State { arguments, cfg::default_value() };
                })
                .map<Application>([](auto impl) {
                    return Application { impl };
                });
    }

    rust::Result<int> Application::operator()() const
    {
        auto commands = output::from_json(impl_->arguments.input.c_str());

        return rust::Err(std::runtime_error("TODO"));
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
