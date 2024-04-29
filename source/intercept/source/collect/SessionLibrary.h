/*  Copyright (C) 2012-2024 by László Nagy
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
        LibraryPreloadSession(bool verbose, const std::string_view &library, const std::string_view &executor);

        static rust::Result<Session::Ptr> from(const flags::Arguments&);

        [[nodiscard]] rust::Result<ic::Execution> resolve(const ic::Execution &execution) const override;
        [[nodiscard]] sys::Process::Builder supervise(const ic::Execution &execution) const override;

        NON_DEFAULT_CONSTRUCTABLE(LibraryPreloadSession)
        NON_COPYABLE_NOR_MOVABLE(LibraryPreloadSession)

    private:
        [[nodiscard]] std::map<std::string, std::string> update(const std::map<std::string, std::string>& env) const;

    private:
        bool verbose_;
        std::string library_;
        std::string executor_;
    };
}
