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

#include "report/wrapper/RpcClients.h"
#include "Convert.h"

#include <fmt/format.h>
#include <grpcpp/create_channel.h>
#include <spdlog/spdlog.h>

namespace {

    std::runtime_error create_error(const grpc::Status& status) {
        return std::runtime_error(fmt::format("gRPC call failed: {}", status.error_message().data()));
    }
}

namespace wr {

    SupervisorClient::SupervisorClient(const SessionLocator &session_locator)
            : channel_(grpc::CreateChannel(session_locator, grpc::InsecureChannelCredentials()))
            , supervisor_(rpc::Supervisor::NewStub(channel_))
    { }

    rust::Result<wr::Execution> SupervisorClient::resolve(const wr::Execution &execution) {
        spdlog::debug("gRPC call requested: supervise::Supervisor::Resolve");

        grpc::ClientContext context;
        rpc::ResolveRequest request;
        rpc::ResolveResponse response;

        request.set_allocated_execution(new rpc::Execution(into(execution)));

        const grpc::Status status = supervisor_->Resolve(&context, request, &response);
        spdlog::debug("gRPC call [Resolve] finished: {}", status.ok());
        return status.ok()
               ? rust::Result<wr::Execution>(rust::Ok(from(response.execution())))
               : rust::Result<wr::Execution>(rust::Err(create_error(status)));
    }

    InterceptorClient::InterceptorClient(const SessionLocator &session_locator)
            : channel_(grpc::CreateChannel(session_locator, grpc::InsecureChannelCredentials()))
            , interceptor_(rpc::Interceptor::NewStub(channel_))
    { }

    rust::Result<int> InterceptorClient::report(const rpc::Event &event) {
        spdlog::debug("gRPC call requested: supervise::Interceptor::Register");

        grpc::ClientContext context;
        google::protobuf::Empty response;

        const grpc::Status status = interceptor_->Register(&context, event, &response);
        spdlog::debug("gRPC call [Register] finished: {}", status.ok());
        if (!status.ok()) {
            return rust::Result<int>(rust::Err(create_error(status)));
        }
        return rust::Result<int>(rust::Ok(0));
    }
}
