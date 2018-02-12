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

#include "Result.h"

struct State {
    char *library;
    char *target;
    char **command;
};

Result<State> parse(int argc, char *argv[]) {
    State result = {nullptr, nullptr, nullptr};

    int opt;
    while ((opt = getopt(argc, argv, "l:t:")) != -1) {
        switch (opt) {
            case 'l':
                result.library = optarg;
                break;
            case 't':
                result.target = optarg;
                break;
            default: /* '?' */
                return Result<State>::failure(
                        // todo: get process name from `argv[0]`.
                        "Usage: pear [-t target_url] [-l path_to_libear] command"
                );
        }
    }

    if (optind >= argc) {
        return Result<State>::failure(
                "Expected argument after options"
        );
    } else {
        result.command = argv + optind;
    }

    return Result<State>::success(result);
}

int main(int argc, char *argv[], char *envp[]) {
    const Result<State> &args = parse(argc, argv);

    args.handle_with([](const char *const message) {
        fprintf(stderr, "%s\n", message);
        exit(EXIT_FAILURE);
    });

    exit(EXIT_SUCCESS);
}
