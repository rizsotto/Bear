/*  Copyright (C) 2012-2017 by László Nagy
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

#include "Interface.h"
#include "Result.h"
#include "Environment.h"

namespace pear {

    /// Used by `intercept-cc` to report single execution.
    struct Session {
        ::pear::Context context_;
        ::pear::Execution execution_;

        Session(const ::pear::Context &context, const ::pear::Execution &execution)
                : context_(context)
                , execution_(execution)
        { }

        virtual ~Session() noexcept = default;

        virtual void configure(::pear::Environment::Builder &builder) const noexcept;
    };

    /// Used by `intercept-build` and `libexec` to report execution
    /// and prepare for more executions.
    struct LibrarySession : public ::pear::Session {
        const char *library;

        LibrarySession(const ::pear::Context &context, const ::pear::Execution &execution)
                : Session(context, execution)
                , library(nullptr)
        { }

        ~LibrarySession() noexcept override = default;

        void configure(::pear::Environment::Builder &builder) const noexcept override;
    };

    /// Used by `intercept-build` to report single execution
    /// and prepare for `intercept-cc`.
    struct WrapperSession : public ::pear::Session {
        const char *cc;
        const char *cxx;
        const char *cc_wrapper;
        const char *cxx_wrapper;

        WrapperSession(const ::pear::Context &context, const ::pear::Execution &execution)
                : Session(context, execution)
                , cc(nullptr)
                , cxx(nullptr)
                , cc_wrapper(nullptr)
                , cxx_wrapper(nullptr)
        { }

        ~WrapperSession() noexcept override = default;

        void configure(::pear::Environment::Builder &builder) const noexcept override;
    };


    using SessionPtr = std::shared_ptr<Session>;
    pear::Result<pear::SessionPtr> parse(int argc, char *argv[]) noexcept;

}
