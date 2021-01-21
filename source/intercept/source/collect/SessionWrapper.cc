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

#include "collect/SessionWrapper.h"
#include "intercept/Flags.h"
#include "report/libexec/Resolver.h"
#include "report/libexec/Environment.h"
#include "report/wrapper/Environment.h"
#include "libsys/Errors.h"
#include "libsys/Os.h"
#include "libsys/Path.h"

#include <spdlog/spdlog.h>
#include <spdlog/fmt/ostr.h>

#include <algorithm>
#include <iterator>
#include <utility>

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

    struct MapHolder {
        const std::map<std::string, std::string>& values;
    };

    std::ostream& operator<<(std::ostream& os, const MapHolder& arguments)
    {
        os << '[';
        for (auto it = arguments.values.begin(); it != arguments.values.end(); ++it) {
            if (it != arguments.values.begin()) {
                os << ", ";
            }
            os << "{ \"" << it->first << "\": \"" << it->second << "\" }";
        }
        os << ']';

        return os;
    }

    rust::Result<std::list<fs::path>> list_dir(const fs::path& path)
    {
        std::list<fs::path> result;

        std::error_code error_code;
        for (auto& candidate : fs::directory_iterator(path, error_code)) {
            if (error_code) {
                return rust::Err(std::runtime_error(error_code.message()));
            }
            if (candidate.is_regular_file()) {
                result.push_back(candidate.path());
            }
        }
        return rust::Ok(result);
    }
}

namespace ic {

    rust::Result<Session::Ptr> WrapperSession::from(const flags::Arguments &args, const char **envp)
    {
        const bool verbose = args.as_bool(flags::VERBOSE).unwrap_or(false);
        auto wrapper_dir = args.as_string(ic::WRAPPER_DIR);
        auto wrappers = wrapper_dir.and_then<std::list<fs::path>>(list_dir);

        auto mapping = wrappers
            .map<std::map<std::string, std::string>>([&envp](auto wrappers) {
                // Find the executables with the same name from the path.
                std::map<std::string, std::string> result;
                el::Resolver resolver;
                for (const auto& wrapper : wrappers) {
                    auto basename = wrapper.filename();
                    auto candidate = resolver.from_path(basename.c_str(), envp);
                    candidate.on_success([&result, &basename](auto candidate) {
                        result[basename] = candidate;
                    });
                }
                return result;
            });

        return rust::merge(wrapper_dir, mapping)
            .map<Session::Ptr>([&envp, &verbose](const auto &tuple) {
                const auto& [wrapper_dir, const_mapping] = tuple;

                std::map<std::string, std::string> mapping(const_mapping);
                std::map<std::string, std::string> override;
                el::Resolver resolver;
                // check if any environment variable is naming the real compiler
                for (auto implicit : IMPLICITS) {
                    // find any of the implicit defined in environment.
                    if (auto env_it = el::env::get_env_value(envp, implicit.env); env_it != nullptr) {
                        // FIXME: it would be more correct if we shell-split the `env_it->second`
                        //        and use only the program name, but not the argument. But then how
                        //        to deal with the errors?
                        auto program = resolver.from_path(std::string_view(env_it), envp);

                        // find the current mapping for the program the user wants to run.
                        // and replace the program what the wrapper will call.
                        if (auto mapping_it = mapping.find(implicit.wrapper); mapping_it != mapping.end()) {
                            program.on_success([&mapping_it](auto path) {
                                mapping_it->second = path;
                            });
                            override[implicit.env] = mapping_it->first;
                        } else {
                            program.on_success([&mapping, &implicit](auto path) {
                                mapping[implicit.wrapper] = path;
                            });
                            override[implicit.env] = implicit.wrapper;
                        }
                    }
                }
                return std::make_shared<WrapperSession>(verbose, std::string(wrapper_dir), std::move(mapping), std::move(override));
            });
    }

    WrapperSession::WrapperSession(
        bool verbose,
        std::string wrapper_dir,
        std::map<std::string, std::string> mapping,
        std::map<std::string, std::string> override)
            : Session()
            , verbose_(verbose)
            , wrapper_dir_(std::move(wrapper_dir))
            , mapping_(std::move(mapping))
            , override_(std::move(override))
    {
        spdlog::debug("session initialized with: wrapper_dir: {}", wrapper_dir_);
        spdlog::debug("session initialized with: mapping: {}", MapHolder { mapping_ });
        spdlog::debug("session initialized with: override: {}", MapHolder { override_ });
    }

    rust::Result<ic::Execution> WrapperSession::resolve(const ic::Execution &execution) const
    {
        spdlog::debug("trying to resolve for wrapper: {}", execution.executable.string());
        return resolve(execution.executable)
                .map<ic::Execution>([this, &execution](auto executable) {
                    auto arguments = execution.arguments;
                    arguments.front() = executable;
                    return ic::Execution{
                            fs::path(executable),
                            std::move(arguments),
                            fs::path(execution.working_dir),
                            update(execution.environment)
                    };
                });
    }

    sys::Process::Builder WrapperSession::supervise(const ic::Execution &execution) const
    {
        auto result = sys::Process::Builder(execution.executable)
                .add_arguments(execution.arguments.begin(), execution.arguments.end())
                .set_environment(set_up(execution.environment));

        return result;
    }

    rust::Result<std::string> WrapperSession::resolve(const std::string& name) const
    {
        if (auto candidate = mapping_.find(name); candidate != mapping_.end()) {
            return rust::Ok(candidate->second);
        }
        return rust::Err(std::runtime_error("not recognized wrapper"));
    }

    std::map<std::string, std::string> WrapperSession::update(const std::map<std::string, std::string>& env) const
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
        return copy;
    }

    std::map<std::string, std::string> WrapperSession::set_up(const std::map<std::string, std::string>& env) const
    {
        std::map<std::string, std::string> environment(env);
        // enable verbose logging to wrappers
        if (verbose_) {
            environment[wr::env::KEY_VERBOSE] = "true";
        }
        // sets the server address to wrappers
        environment[wr::env::KEY_DESTINATION] = *session_locator_;
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
