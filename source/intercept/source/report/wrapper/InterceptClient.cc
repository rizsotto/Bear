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

#include "InterceptClient.h"

#include <fmt/format.h>
#include <grpcpp/create_channel.h>
#include <spdlog/spdlog.h>

namespace {

    std::runtime_error create_error(const grpc::Status& status)
    {
        return std::runtime_error(fmt::format("gRPC call failed: {}", status.error_message().data()));
    }
}

namespace rpc {

    InterceptClient::InterceptClient(const Session& session)
            : channel_(grpc::CreateChannel(session.destination, grpc::InsecureChannelCredentials()))
            , supervisor_(rpc::Supervisor::NewStub(channel_))
            , interceptor_(rpc::Interceptor::NewStub(channel_))
    {
    }

    rust::Result<std::string> InterceptClient::get_wrapped_command(const std::string& name)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::GetWrappedCommand");

        grpc::ClientContext context;
        rpc::ResolveRequest request;
        rpc::ResolveResponse response;

        request.set_path(name);

        const grpc::Status status = supervisor_->ResolveProgram(&context, request, &response);
        spdlog::debug("gRPC call [ResolveProgram] finished: {}", status.ok());
        return status.ok()
            ? rust::Result<std::string>(rust::Ok(response.path()))
            : rust::Result<std::string>(rust::Err(create_error(status)));
    }

    rust::Result<std::map<std::string, std::string>> InterceptClient::get_environment_update(const std::map<std::string, std::string>& input)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::GetEnvironmentUpdate");

        grpc::ClientContext context;
        rpc::Environment request;
        rpc::Environment response;

        request.mutable_values()->insert(input.begin(), input.end());

        const grpc::Status status = supervisor_->Update(&context, request, &response);
        spdlog::debug("gRPC call [Update] finished: {}", status.ok());
        if (status.ok()) {
            std::map<std::string, std::string> copy(response.values().begin(), response.values().end());
            return rust::Ok(copy);
        }
        return rust::Err(create_error(status));
    }

    rust::Result<int> InterceptClient::report(rpc::Event&& event)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::Report");

        grpc::ClientContext context;
        rpc::Empty response;

        const grpc::Status status = interceptor_->Register(&context, event, &response);
        spdlog::debug("gRPC call [Register] finished: {}", status.ok());
        if (!status.ok()) {
            return rust::Result<int>(rust::Err(create_error(status)));
        }
        return rust::Result<int>(rust::Ok(0));
    }
}
