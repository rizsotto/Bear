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

#include "report/wrapper/Types.h"
#include "report/wrapper/EventFactory.h"
#include "libresult/Result.h"

#include <grpcpp/channel.h>
#include "intercept.grpc.pb.h"
#include "supervise.grpc.pb.h"

namespace wr {

    class SupervisorClient {
    public:
        explicit SupervisorClient(const Session& session);

        SupervisorClient() = delete;
        SupervisorClient(const SupervisorClient&) = delete;
        SupervisorClient(SupervisorClient&&) noexcept = delete;

        SupervisorClient& operator=(const SupervisorClient&) = delete;
        SupervisorClient& operator=(SupervisorClient&&) noexcept = delete;

    public:
        rust::Result<wr::Execution> resolve(const wr::Execution &execution);

    private:
        std::shared_ptr<::grpc::Channel> channel_;
        std::unique_ptr<rpc::Supervisor::Stub> supervisor_;
    };

    class InterceptorClient {
    public:
        explicit InterceptorClient(const Session& session);

        InterceptorClient() = delete;
        InterceptorClient(const InterceptorClient&) = delete;
        InterceptorClient(InterceptorClient&&) noexcept = delete;

        InterceptorClient& operator=(const InterceptorClient&) = delete;
        InterceptorClient& operator=(InterceptorClient&&) noexcept = delete;

    public:
        rust::Result<int> report(rpc::Event&&);

    private:
        std::shared_ptr<::grpc::Channel> channel_;
        std::unique_ptr<rpc::Interceptor::Stub> interceptor_;
    };
}
