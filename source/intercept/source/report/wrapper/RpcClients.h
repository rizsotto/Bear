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

#pragma once

#include "config.h"
#include "Domain.h"
#include "report/wrapper/EventFactory.h"
#include "libresult/Result.h"

#include <memory>

#include <grpcpp/channel.h>
#include "intercept.grpc.pb.h"
#include "supervise.grpc.pb.h"

namespace wr {
    using namespace domain;

    class SupervisorClient {
    public:
        explicit SupervisorClient(const wr::SessionLocator& session_locator);

        rust::Result<wr::Execution> resolve(const wr::Execution &);

        NON_DEFAULT_CONSTRUCTABLE(SupervisorClient)
        NON_COPYABLE_NOR_MOVABLE(SupervisorClient)

    private:
        std::shared_ptr<::grpc::Channel> channel_;
        std::unique_ptr<rpc::Supervisor::Stub> supervisor_;
    };

    class InterceptorClient {
    public:
        explicit InterceptorClient(const wr::SessionLocator& session_locator);

        rust::Result<int> report(const rpc::Event &);

        NON_DEFAULT_CONSTRUCTABLE(InterceptorClient)
        NON_COPYABLE_NOR_MOVABLE(InterceptorClient)

    private:
        std::shared_ptr<::grpc::Channel> channel_;
        std::unique_ptr<rpc::Interceptor::Stub> interceptor_;
    };
}
