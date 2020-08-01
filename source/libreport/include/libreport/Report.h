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

#pragma once

#include "libresult/Result.h"

#include <filesystem>
#include <list>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <iostream>

namespace fs = std::filesystem;

namespace report {

    // This represents the executed command itself. Describes all the
    // context that the caller was given. And these are the those parameters
    // which are essential for re-run the command.
    struct Command {
        fs::path program;
        std::list<std::string> arguments;
        fs::path working_dir;
        std::map<std::string, std::string> environment;
    };

    // Merged view of all possible events that can happen to a process.
    // An instance can represent a process start event or a stop event
    // (but only one of them).
    //
    // - The `type` attribute tells which event was the source of it.
    // - The `at` attribute tells when that event has happened.
    // - The `status` is present for a stop event, and holds the value
    // of the exit status of the process.
    // - The `signal` is present for a signal event, and holds the value
    // of the signal number that the process received.
    struct Event {
        std::string type;
        std::string at;
        std::optional<int> status;
        std::optional<int> signal;
    };

    // This represents a single run and holds the attributes which are
    // the history of the execution.
    struct Run {
        int pid;
        std::optional<int> ppid;
        std::list<Event> events;
    };

    // This represents a program execution, which is assembled from multiple
    // events. The events are representing a process execution event, like
    // the program is started or the program stopped.
    struct Execution {
        Command command;
        Run run;
    };

    // This represents some basic information about the execution context.
    struct Context {
        // The interception technique which was used to capture the executions.
        std::string session_type;
        // Basic information about the machine operating system. This is useful
        // for bug reporting. Contains results from `uname` and `confstr` system
        // calls.
        std::map<std::string, std::string> host_info;
    };

    // This represents the complete execution report of a single build,
    // produced by the `intercept` command. The execution list is not ordered
    // nor filtered.
    struct Report {
        Context context;
        std::list<Execution> executions;
    };

    // Methods used in tests.
    bool operator==(const Command& lhs, const Command& rhs);
    bool operator==(const Event& lhs, const Event& rhs);
    bool operator==(const Run& lhs, const Run& rhs);
    bool operator==(const Execution& lhs, const Execution& rhs);
    bool operator==(const Context& lhs, const Context& rhs);
    bool operator==(const Report& lhs, const Report& rhs);

    // Utility class to persists entries.
    struct ReportSerializer {
        virtual ~ReportSerializer() noexcept = default;

        // Serialization methods with error mapping.
        virtual rust::Result<int> to_json(const fs::path&, const Report& rhs) const;
        virtual rust::Result<int> to_json(std::ostream& ostream, const Report& rhs) const;

        virtual rust::Result<Report> from_json(const fs::path&) const;
        virtual rust::Result<Report> from_json(std::istream& istream) const;
    };
}
