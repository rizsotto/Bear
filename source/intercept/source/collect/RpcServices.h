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

#include "intercept.grpc.pb.h"
#include "supervise.grpc.pb.h"

namespace ic {

    class Reporter;
    class Session;

    class SupervisorImpl final : public rpc::Supervisor::Service {
    public:
        explicit SupervisorImpl(const Session&);
        ~SupervisorImpl() override = default;

        grpc::Status Resolve(grpc::ServerContext *context, const rpc::ResolveRequest *request, rpc::ResolveResponse *response) override;

    private:
        const Session &session_;
    };

    class InterceptorImpl final : public rpc::Interceptor::Service {
    public:
        explicit InterceptorImpl(Reporter&);
        ~InterceptorImpl() override = default;

        ::grpc::Status Register(::grpc::ServerContext* context, const rpc::Event* request, rpc::Empty* response) override;

    private:
        Reporter& reporter_;
    };
}
