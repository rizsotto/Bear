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

#include <grpc/grpc.h>
#include <grpcpp/server.h>
#include <grpcpp/server_builder.h>
#include <grpcpp/server_context.h>
#include <grpcpp/security/server_credentials.h>

#include <thread>
#include <memory>
#include <vector>

namespace ic {

    ::rust::Result<Command> Command::create(const ::flags::Arguments& args)
    {
        ReporterPtr reporter = std::make_shared<Reporter>();
        SessionConstPtr session = std::shared_ptr<const Session>(new FakeSession());
        return rust::Ok<Command>({ reporter, session });
    }

    ::rust::Result<int> Command::operator()()
    {
        //    InterceptorImpl server_;
        ::grpc::ServerBuilder builder;
        //    std::thread supervisor_;
        //    std::thread interceptor_;
        return rust::Ok<int>(0);
    }

    Command::Command(ReporterPtr reporter, SessionConstPtr session)
            : reporter_(reporter)
            , session_(session)
    {
    }
}
