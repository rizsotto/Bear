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

#include "collect/Session.h"

namespace ic {

    class WrapperSession : public ic::Session {
    public:
        WrapperSession(
            bool verbose,
            std::string&& wrapper_dir,
            std::map<std::string, std::string>&& mapping,
            std::map<std::string, std::string>&& override,
            const sys::env::Vars& environment);

        static rust::Result<Session::SharedPtr> from(const flags::Arguments&, sys::env::Vars &&);

    public:
        [[nodiscard]] rust::Result<std::string> resolve(const std::string& name) const override;
        [[nodiscard]] rust::Result<std::map<std::string, std::string>> update(const std::map<std::string, std::string>& env) const override;
        [[nodiscard]] rust::Result<sys::Process::Builder> supervise(const std::vector<std::string_view>& command) const override;

        [[nodiscard]] std::string get_session_type() const override;

    private:
        [[nodiscard]] std::map<std::string, std::string> set_up_environment() const;

    private:
        bool verbose_;
        std::string wrapper_dir_;
        std::map<std::string, std::string> mapping_;
        std::map<std::string, std::string> override_;
        sys::env::Vars environment_;
    };
}
