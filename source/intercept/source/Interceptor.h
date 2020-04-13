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

#include "librpc/supervise.grpc.pb.h"

namespace ic {

    class Reporter;
    class Session;

    class InterceptorImpl final : public ::supervise::Interceptor::Service {
    public:
        InterceptorImpl(Reporter&, const Session&);
        ~InterceptorImpl() override = default;

        ::grpc::Status GetWrappedCommand(::grpc::ServerContext* context, const ::supervise::WrapperRequest* request, ::supervise::WrapperResponse* response) override;
        ::grpc::Status GetEnvironmentUpdate(::grpc::ServerContext* context, const ::supervise::EnvironmentRequest* request, ::supervise::EnvironmentResponse* response) override;
        ::grpc::Status Report(::grpc::ServerContext* context, ::grpc::ServerReader<::supervise::Event>* reader, ::supervise::Empty* response) override;

    private:
        Reporter& reporter_;
        const Session& session_;
    };
}
