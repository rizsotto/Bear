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

#include "Command.h"

#include "Interceptor.h"
#include "Reporter.h"
#include "Session.h"

#include <grpc/grpc.h>
#include <grpcpp/server.h>
#include <grpcpp/server_builder.h>
#include <grpcpp/server_context.h>
#include <grpcpp/security/server_credentials.h>

#include <thread>
#include <memory>
#include <vector>

namespace ic {

    struct Command::State {
        ReporterPtr reporter_;
        SessionConstPtr session_;
    };

    ::rust::Result<Command> Command::create(const ::flags::Arguments& args)
    {
        auto reporter = std::make_shared<Reporter>();
        auto session = std::shared_ptr<const Session>(new FakeSession());
        auto impl = new Command::State { reporter, session };
        return rust::Ok<Command>(Command { impl });
    }

    ::rust::Result<int> Command::operator()() const
    {
        //    InterceptorImpl server_;
        ::grpc::ServerBuilder builder;
        //    std::thread supervisor_;
        //    std::thread interceptor_;
        return rust::Ok<int>(0);
    }

    Command::Command(Command::State* const impl)
            : impl_(impl)
    {
    }

    Command::Command(Command&& rhs) noexcept
            : impl_(rhs.impl_)
    {
        rhs.impl_ = nullptr;
    }

    Command& Command::operator=(Command&& rhs) noexcept
    {
        if (&rhs != this) {
            delete impl_;
            impl_ = rhs.impl_;
        }
        return *this;
    }

    Command::~Command()
    {
        delete impl_;
        impl_ = nullptr;
    }
}
