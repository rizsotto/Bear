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

        return pear::spawnp(config.command, environment->as_array());
    }

    void report_start(pid_t pid, const char **cmd, pear::ReporterPtr reporter) noexcept {
        pear::Event::start(pid, cmd)
                .map<int>([&reporter](const pear::EventPtr &eptr) {
                    return reporter->send(eptr)
                            .handle_with([](auto message) {
                                fprintf(stderr, "%s\n", message.what());
                            })
                            .get_or_else(0);
                });
    }

    void report_exit(pid_t pid, int exit, pear::ReporterPtr reporter) noexcept {
        pear::Event::stop(pid, exit)
                .map<int>([&reporter](const pear::EventPtr &eptr) {
                    return reporter->send(eptr)
                            .handle_with([](auto message) {
                                fprintf(stderr, "%s\n", message.what());
                            })
                            .get_or_else(0);
                });
    }

}

int main(int argc, char *argv[], char *envp[]) {
    return ::pear::parse(argc, argv, envp)
            .bind<int>([&envp](auto &state) {
                auto builder = pear::Environment::Builder(const_cast<const char **>(envp));
                auto environment = state->set(builder).build();
                auto reporter = pear::Reporter::tempfile(state->context.destination);

                pear::Result<pid_t> child = spawnp(state->execution, environment);
                return child.map<int>([&reporter, &state](auto &pid) {
                    report_start(pid, state->execution.command, reporter);
                    pear::Result<int> status = pear::wait_pid(pid);
                    return status
                            .map<int>([&reporter, &pid](auto &exit) {
                                report_exit(pid, exit, reporter);
                                return exit;
                            })
                            .get_or_else(EXIT_FAILURE);
                });
            })
            .handle_with([](auto const &message) {
                fprintf(stderr, "%s\n", message.what());
                exit(EXIT_FAILURE);
            })
            .get_or_else(EXIT_FAILURE);
}
