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

#include <memory>

#include "Environment.h"
#include "Interface.h"
#include "Result.h"

namespace er {

    /// Used by `intercept-cc` to report single execution.
    struct Session {
        ::er::Context context_;
        ::er::Execution execution_;

        Session(const ::er::Context& context, const ::er::Execution& execution)
                : context_(context)
                , execution_(execution)
        {
        }

        virtual ~Session() noexcept = default;

        virtual void configure(::er::Environment::Builder& builder) const noexcept;
    };

    /// Used by `intercept-build` and `libexec` to report execution
    /// and prepare for more executions.
    struct LibrarySession : public ::er::Session {
        const char* library;

        LibrarySession(const ::er::Context& context, const ::er::Execution& execution)
                : Session(context, execution)
                , library(nullptr)
        {
        }

        ~LibrarySession() noexcept override = default;

        void configure(::er::Environment::Builder& builder) const noexcept override;
    };

    using SessionPtr = std::shared_ptr<Session>;
    rust::Result<er::SessionPtr> parse(int argc, char* argv[]) noexcept;

}
