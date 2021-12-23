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

#include "Domain.h"
#include "Convert.h"

#include <google/protobuf/util/json_util.h>

#include <iostream>

namespace domain {

    bool operator==(const Execution &lhs, const Execution &rhs) {
        return (lhs.executable == rhs.executable)
               && (lhs.arguments == rhs.arguments)
               && (lhs.working_dir == rhs.working_dir)
               && (lhs.environment == rhs.environment);
    }

    std::ostream &operator<<(std::ostream &os, const Execution &rhs) {
        const auto rpc = into(rhs);
        std::string json;
        const auto rc = google::protobuf::util::MessageToJsonString(rpc, &json);
        if (rc.ok()) {
            os << json;
        }
        return os;
    }

    bool operator==(const Run &lhs, const Run &rhs) {
        return (lhs.execution == rhs.execution)
               && (lhs.pid == rhs.pid)
               && (lhs.ppid == rhs.ppid);
    }

    std::ostream &operator<<(std::ostream &os, const Run &rhs) {
        os << std::boolalpha;
        os << R"({"execution": })" << rhs.execution
            << R"(, "pid": )" << rhs.pid
            << R"(, "ppid": )" << rhs.ppid
            << R"(})";
        return os;
    }
}
