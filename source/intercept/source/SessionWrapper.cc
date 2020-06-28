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

#include "SessionWrapper.h"

#include "Application.h"
#include "libwrapper/Environment.h"
#include "libsys/Context.h"
#include "libsys/Path.h"

#include <algorithm>
#include <iterator>

namespace {

    struct Rule {
        const char* env;
        const char* wrapper;
    };

    // The list of implicit rules for the build systems.
    //
    // The environment variable names an executable (or an executable plus an
    // argument) which will be run for a given build step.
    //
    // NOTES: current implementation depends on the list has unique environment
    //        names, but also unique wrapper names too.
    //
    // https://www.gnu.org/software/make/manual/html_node/Implicit-Variables.html
    constexpr Rule IMPLICITS[] = {
        { "AR", "ar" },
        { "AS", "as" },
        { "CC", "cc" },
        { "CXX", "c++" },
        { "CPP", "cpp" },
        { "FC", "f77" },
        { "M2C", "m2c" },
        { "PC", "pc" },
        { "LEX", "lex" },
        { "YACC", "yacc" },
        { "LINT", "lint" },
        { "MAKEINFO", "makeinfo" },
        { "TEX", "tex" },
        { "TEXI2DVI", "texi2dvi" },
        { "WEAVE", "weave" },
        { "CWEAVE", "cweave" },
        { "TANGLE", "tangle" },
        { "CTANGLE", "ctangle" }
    };
}

namespace ic {

    rust::Result<Session::SharedPtr> WrapperSession::from(const flags::Arguments& args, const sys::Context& ctx)
    {
        auto environment = ctx.get_environment();
        auto path = ctx.get_path();

        auto verbose = args.as_bool(ic::Application::VERBOSE)
                           .unwrap_or(false);

        auto wrapper_dir = args.as_string(ic::Application::WRAPPER);

        auto wrappers = args.as_string(ic::Application::WRAPPER)
                            .and_then<std::list<std::string>>([&ctx](auto wrapper_dir) {
                                return ctx.list_dir(wrapper_dir);
                            });

        auto mapping_and_override = rust::merge(path, wrappers)
            .map<std::map<std::string, std::string>>([&ctx](auto tuple) {
                const auto& [paths, wrappers] = tuple;
                // Find the executables with the same name from the path.
                std::map<std::string, std::string> result = {};
                for (const auto& wrapper : wrappers) {
                    auto basename = sys::path::basename(wrapper);
                    for (const auto& path : paths) {
                        auto candidate = sys::path::concat(path, basename);
                        bool executable = ctx.real_path(candidate)
                                             .template map<bool>([&ctx](auto path) {
                                                 return 0 == ctx.is_executable(path);
                                             })
                                             .unwrap_or(false);
                        if (executable) {
                            result[wrapper] = candidate;
                            break;
                        }
                    }
                }
                return result;
            })
            .map<std::tuple<std::map<std::string, std::string>, std::map<std::string, std::string>>>([&environment](auto mapping) {
                std::map<std::string, std::string> override;
                // check if any environment variable is naming the real compiler
                for (auto implicit : IMPLICITS) {
                    // find any of the implicit defined in environment.
                    if (auto env_it = environment.find(implicit.env); env_it != environment.end()) {
                        // find the current mapping for the program the user wants to run.
                        auto mapping_it = std::find_if(mapping.begin(), mapping.end(), [&implicit](auto value) {
                            return sys::path::basename(value.first) == implicit.wrapper;
                        });
                        // replace the program what the wrapper will call.
                        if (mapping_it != mapping.end()) {
                            auto program = env_it->second;
                            auto argument = nullptr;
                            // FIXME: it would be more correct if we shell-split the `env_it->second`
                            //        and use only the program name, but not the argument.
                            sys::Process::Builder(program)
                                .resolve_executable()
                                .on_success([&mapping_it](auto path) {
                                    mapping_it->second = path;
                                });
                            override[implicit.env] = (argument == nullptr)
                                ? mapping_it->first
                                : fmt::format("{} {}", mapping_it->first, argument);
                        }
                    }
                }
                return std::make_tuple(mapping, override);
            });

        return rust::merge(wrapper_dir, mapping_and_override)
            .map<Session::SharedPtr>([&verbose, &environment](const auto& tuple) {
                const auto& [const_wrapper_dir, const_mapping_and_override] = tuple;
                const auto& [const_mapping, const_override] = const_mapping_and_override;
                std::string wrapper_dir(const_wrapper_dir);
                std::map<std::string, std::string> mapping(const_mapping);
                std::map<std::string, std::string> override(const_override);
                return std::make_shared<WrapperSession>(verbose, std::move(wrapper_dir), std::move(mapping), std::move(override), std::move(environment));
            });
    }

    WrapperSession::WrapperSession(
        bool verbose,
        std::string&& wrapper_dir,
        std::map<std::string, std::string>&& mapping,
        std::map<std::string, std::string>&& override,
        std::map<std::string, std::string>&& environment)
            : Session()
            , verbose_(verbose)
            , wrapper_dir_(wrapper_dir)
            , mapping_(mapping)
            , override_(override)
            , environment_(environment)
    {
    }

    rust::Result<std::string> WrapperSession::resolve(const std::string& name) const
    {
        auto candidate = mapping_.find(name);
        return (candidate != mapping_.end())
                ? rust::Result<std::string>(rust::Ok(candidate->second))
                : rust::Result<std::string>(rust::Err(std::runtime_error("TODO")));
    }

    rust::Result<std::map<std::string, std::string>> WrapperSession::update(const std::map<std::string, std::string>& env) const
    {
        std::map<std::string, std::string> copy(env);

        // remove wrapper directory from path
        if (auto it = copy.find("PATH"); it != copy.end()) {
            it->second = remove_from_path(wrapper_dir_, it->second);
        }
        // remove verbose flag
        if (auto it = copy.find(wr::env::KEY_VERBOSE); it != copy.end()) {
            copy.erase(it);
        }
        // remove destination
        if (auto it = copy.find(wr::env::KEY_DESTINATION); it != copy.end()) {
            copy.erase(it);
        }
        // remove all implicits
        for (const auto& override : override_) {
            if (auto it = copy.find(override.first); it != copy.end()) {
                copy.erase(it);
            }
        }

        return rust::Ok(copy);
    }

    rust::Result<sys::Process::Builder> WrapperSession::supervise(const std::vector<std::string_view>& command) const
    {
        return rust::Ok(
            sys::Process::Builder(command.front())
                .add_arguments(command.begin(), command.end())
                .set_environment(set_up_environment()));
    }

    std::string WrapperSession::get_session_type() const
    {
        return std::string("Wrapper");
    }

    std::map<std::string, std::string> WrapperSession::set_up_environment() const
    {
        std::map<std::string, std::string> environment(environment_);
        // enable verbose logging to wrappers
        if (verbose_) {
            environment[wr::env::KEY_VERBOSE] = "true";
        }
        // sets the server address to wrappers
        environment[wr::env::KEY_DESTINATION] = server_address_;
        // change PATH to put the wrapper directory at the front.
        if (auto it = environment.find("PATH"); it != environment.end()) {
            it->second = keep_front_in_path(wrapper_dir_, it->second);
        }
        // replace all implicit program to the wrapper
        for (const auto& it : override_) {
            environment[it.first] = it.second;
        }
        return environment;
    }
}
