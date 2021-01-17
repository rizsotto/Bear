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

    class LibraryPreloadSession : public ic::Session {
    public:
        LibraryPreloadSession(
                const std::string_view &library,
                const std::string_view &executor,
                bool verbose,
                const std::string &path,
                sys::env::Vars &&environment
        );

        static rust::Result<Session::Ptr> from(const flags::Arguments&, const char **envp);

    public:
        [[nodiscard]] rust::Result<ic::Execution> resolve(const ic::Execution &input) const override;
        [[nodiscard]] rust::Result<sys::Process::Builder> supervise(const std::vector<std::string_view>& command) const override;

    private:
        [[nodiscard]] std::map<std::string, std::string> update(const std::map<std::string, std::string>& env) const;

    private:
        std::string library_;
        std::string executor_;
        std::string path_;
        bool verbose_;
        sys::env::Vars environment_;
    };
}
