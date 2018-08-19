/*  Copyright (C) 2012-2017 by László Nagy
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

#include <unistd.h>

#include <cstdlib>
#include <cstdio>
#include <cstring>

#include "intercept_a/Interface.h"
#include "intercept_a/Session.h"
#include "intercept_a/Result.h"
#include "intercept_a/Environment.h"
#include "intercept_a/Reporter.h"
#include "intercept_a/SystemCalls.h"


namespace {

    pear::Result<pid_t> spawnp(const ::pear::Execution &config,
                               const ::pear::EnvironmentPtr &environment) noexcept {
        // TODO: use other execution config parameters.

        return pear::spawnp(config.command, environment->data());
    }

    void report_start(::pear::Result<pear::ReporterPtr> const &reporter, pid_t pid, const char **cmd) noexcept {
        ::pear::merge(reporter, ::pear::Event::start(pid, cmd))
                .bind<int>([](auto tuple) {
                    auto [ rptr, eptr ] = tuple;
                    return rptr->send(eptr);
                })
                .handle_with([](auto message) {
                    fprintf(stderr, "%s\n", message.what());
                })
                .get_or_else(0);
    }

    void report_exit(::pear::Result<pear::ReporterPtr> const &reporter, pid_t pid, int exit) noexcept {
        ::pear::merge(reporter, ::pear::Event::stop(pid, exit))
                .bind<int>([](auto tuple) {
                    auto [ rptr, eptr ] = tuple;
                    return rptr->send(eptr);
                })
                .handle_with([](auto message) {
                    fprintf(stderr, "%s\n", message.what());
                })
                .get_or_else(0);
    }

}

int main(int argc, char *argv[], char *envp[]) {
    return ::pear::parse(argc, argv)
            .bind<int>([&envp](auto &state) {
                auto builder = pear::Environment::Builder(const_cast<const char **>(envp));
                auto environment = state->set(builder).build();

                pear::Result<pid_t> child = spawnp(state->execution_, environment);
                return child.map<int>([&state](auto &pid) {
                    auto reporter = pear::Reporter::tempfile(state->context_.destination);
                    report_start(reporter, pid, state->execution_.command);

                    pear::Result<int> status = pear::wait_pid(pid);
                    return status
                            .map<int>([&reporter, &pid](auto &exit) {
                                report_exit(reporter, pid, exit);
                                return exit;
                            })
                            .get_or_else(EXIT_FAILURE);
                });
            })
            .handle_with([](auto const &message) {
                fprintf(stderr, "%s\n", message.what());
            })
            .get_or_else(EXIT_FAILURE);
}
