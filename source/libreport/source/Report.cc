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

#include "libreport/Report.h"

using json = nlohmann::json;

namespace report {

    void to_json(json& j, const Execution::Command& rhs)
    {
        j = json {
            { "program", rhs.program },
            { "arguments", json(rhs.arguments) },
            { "working_dir", rhs.working_dir },
            { "environment", json(rhs.environment) }
        };
    }

    void to_json(json& j, const Execution::Event& rhs)
    {
        j = json {
            { "at", rhs.at },
            { "type", rhs.type }
        };
        if (rhs.status) {
            j["status"] = rhs.status.value();
        }
        if (rhs.signal) {
            j["signal"] = rhs.signal.value();
        }
    }

    void to_json(json& j, const Execution::Run& rhs)
    {
        j["pid"] = rhs.pid;
        j["events"] = json(rhs.events);
        if (rhs.ppid) {
            j["ppid"] = rhs.ppid.value();
        }
    }

    void to_json(json& j, const Execution& rhs)
    {
        j = json { { "command", rhs.command }, { "run", rhs.run } };
    }

    void to_json(json& j, const Context& rhs)
    {
        j = json {
            { "intercept", rhs.session_type },
            { "host_info", json(rhs.host_info) }
        };
    }

    void to_json(json& j, const Report& rhs)
    {
        j = json { { "executions", rhs.executions }, { "context", rhs.context } };
    }

    bool operator==(const Execution::Command& lhs, const Execution::Command& rhs)
    {
        return (lhs.program == rhs.program)
               && (lhs.arguments == rhs.arguments)
               && (lhs.working_dir == rhs.working_dir)
               && (lhs.environment == rhs.environment);
    }

    bool operator==(const Execution::Event& lhs, const Execution::Event& rhs)
    {
        return (lhs.at == rhs.at)
               && (lhs.type == rhs.type)
               && (lhs.status == rhs.status)
               && (lhs.signal == rhs.signal);
    }

    bool operator==(const Execution::Run& lhs, const Execution::Run& rhs)
    {
        return (lhs.pid == rhs.pid)
               && (lhs.ppid == rhs.ppid)
               && (lhs.events == rhs.events);
    }

    bool operator==(const Execution& lhs, const Execution& rhs)
    {
        return (lhs.command == rhs.command)
               && (lhs.run == rhs.run);
    }

    bool operator==(const Context& lhs, const Context& rhs)
    {
        return (lhs.session_type == rhs.session_type)
               && (lhs.host_info == rhs.host_info);
    }

    bool operator==(const Report& lhs, const Report& rhs)
    {
        return (lhs.context == rhs.context)
               && (lhs.executions == rhs.executions);
    }
}
