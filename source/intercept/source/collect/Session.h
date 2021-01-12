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
#include <memory>
#include <string>
#include <string_view>

#include "libflags/Flags.h"
#include "libresult/Result.h"
#include "libsys/Os.h"
#include "libsys/Process.h"

namespace ic {

    class Session {
    public:
        using Ptr = std::shared_ptr<Session>;
        static rust::Result<Session::Ptr> from(const flags::Arguments&, const char **envp);

    public:
        virtual ~Session() = default;

        [[nodiscard]] virtual rust::Result<std::string> resolve(const std::string& name) const = 0;
        [[nodiscard]] virtual rust::Result<std::map<std::string, std::string>> update(const std::map<std::string, std::string>& env) const = 0;
        [[nodiscard]] virtual rust::Result<sys::Process::Builder> supervise(const std::vector<std::string_view>& command) const = 0;
        [[nodiscard]] virtual rust::Result<int> execute(const std::vector<std::string_view>& command) const;

        [[nodiscard]] virtual std::string get_session_type() const = 0;

        void set_server_address(const std::string&);

    protected:
        static std::string keep_front_in_path(const std::string& path, const std::string& paths);
        static std::string remove_from_path(const std::string& path, const std::string& paths);

    protected:
        std::string server_address_;
    };
}
