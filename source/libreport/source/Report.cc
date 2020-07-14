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

#include <iomanip>

#include <fstream>
#include <nlohmann/json.hpp>

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

    void from_json(const json& j, Execution::Command& rhs)
    {
        j.at("program").get_to(rhs.program);
        j.at("arguments").get_to(rhs.arguments);
        j.at("working_dir").get_to(rhs.working_dir);
        j.at("environment").get_to(rhs.environment);
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

    void from_json(const json& j, Execution::Event& rhs)
    {
        j.at("at").get_to(rhs.at);
        j.at("type").get_to(rhs.type);

        if (j.contains("status")) {
            int value;
            j.at("status").get_to(value);
            rhs.status.emplace(value);
        }

        if (j.contains("signal")) {
            int value;
            j.at("signal").get_to(value);
            rhs.signal.emplace(value);
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

    void from_json(const json& j, Execution::Run& rhs)
    {
        j.at("pid").get_to(rhs.pid);
        j.at("events").get_to(rhs.events);

        if (j.contains("ppid")) {
            int value;
            j.at("ppid").get_to(value);
            rhs.ppid.emplace(value);
        }
    }

    void to_json(json& j, const Execution& rhs)
    {
        j = json { { "command", rhs.command }, { "run", rhs.run } };
    }

    void from_json(const json& j, Execution& rhs)
    {
        j.at("command").get_to(rhs.command);
        j.at("run").get_to(rhs.run);
    }

    void to_json(json& j, const Context& rhs)
    {
        j = json {
            { "intercept", rhs.session_type },
            { "host_info", json(rhs.host_info) }
        };
    }

    void from_json(const json& j, Context& rhs)
    {
        j.at("intercept").get_to(rhs.session_type);
        j.at("host_info").get_to(rhs.host_info);
    }

    void to_json(json& j, const Report& rhs)
    {
        j = json { { "executions", rhs.executions }, { "context", rhs.context } };
    }

    void from_json(const json& j, Report& rhs)
    {
        j.at("executions").get_to(rhs.executions);
        j.at("context").get_to(rhs.context);
    }

    rust::Result<int> to_json(const char* file, const Report& rhs)
    {
        try {
            std::ofstream target(file);
            return to_json(target, rhs);
        } catch (const std::exception& error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<int> to_json(std::ostream& ostream, const Report& rhs)
    {
        try {
            nlohmann::json out = rhs;
            ostream << std::setw(4) << out << std::endl;

            return rust::Ok(1);
        } catch (const std::exception& error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<Report> from_json(const char* file)
    {
        try {
            std::ifstream source(file);
            return from_json(source);
        } catch (const std::exception& error) {
            return rust::Err(std::runtime_error(error.what()));
        }
    }

    rust::Result<Report> from_json(std::istream& istream)
    {
        try {
            nlohmann::json in;
            istream >> in;

            report::Report result;
            report::from_json(in, result);

            return rust::Ok(result);
        } catch (const std::exception& error) {
            return rust::Err(std::runtime_error(error.what()));
        }
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
