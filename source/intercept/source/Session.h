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

#include <map>
#include <string>

namespace ic {

    class Session {
    public:
        virtual ~Session() = default;

        // TODO: these shall return `Result<>`
        virtual const char* resolve(const std::string& name) const = 0;

        virtual std::map<std::string, std::string>&& update(std::map<std::string, std::string>&& env) const = 0;

        virtual void set_server_address(const std::string&) = 0;
    };

    struct FakeSession : public Session {
        const char* resolve(const std::string& name) const override
        {
            return "null pointer";
        }

        std::map<std::string, std::string>&& update(std::map<std::string, std::string>&& env) const override
        {
            return std::move(env);
        }

        void set_server_address(const std::string &) override
        {
            return;
        }
    };

    using SessionPtr = std::shared_ptr<Session>;
}