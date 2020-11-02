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

#include <mutex>

#include "librpc/supervise.grpc.pb.h"

namespace ic {

    class Reporter;
    class Session;

    class SupervisorImpl final : public ::supervise::Supervisor::Service {
    public:
        explicit SupervisorImpl(const Session&);
        ~SupervisorImpl() override = default;

        ::grpc::Status ResolveProgram(::grpc::ServerContext* context, const ::supervise::ResolveRequest* request, ::supervise::ResolveResponse* response) override;
        ::grpc::Status Update(::grpc::ServerContext* context, const ::supervise::Environment* request, ::supervise::Environment* response) override;

    private:
        const Session& session_;
    };

    class InterceptorImpl final : public ::supervise::Interceptor::Service {
    public:
        explicit InterceptorImpl(Reporter&);
        ~InterceptorImpl() override = default;

        ::grpc::Status Register(::grpc::ServerContext* context, const ::supervise::Event* request, ::supervise::Empty* response) override;

    private:
        Reporter& reporter_;
        std::mutex lock_;
    };
}
