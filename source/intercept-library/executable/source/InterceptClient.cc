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
#include <grpcpp/create_channel.h>
#include <fmt/format.h>
#include <spdlog/spdlog.h>

namespace {

    std::unique_ptr<supervise::Interceptor::Stub> create_stub(const std::string_view& address)
    {
        return supervise::Interceptor::NewStub(
            grpc::CreateChannel(address.data(), grpc::InsecureChannelCredentials()));
    }

    std::runtime_error create_error(const grpc::Status& status)
    {
        return std::runtime_error(fmt::format("gRPC call failed: {}", status.error_message().data()));
    }
}

namespace er {

    InterceptClient::InterceptClient(const std::string_view& address)
            : stub_(create_stub(address))
    {
    }

    rust::Result<std::string> InterceptClient::get_wrapped_command(const std::string& name)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::GetWrappedCommand");

        grpc::ClientContext context;
        supervise::WrapperRequest request;
        supervise::WrapperResponse response;

        request.set_name(name);

        const grpc::Status status = stub_->GetWrappedCommand(&context, request, &response);
        spdlog::debug("gRPC call finished: {}", status.ok());
        return status.ok()
            ? rust::Result<std::string>(rust::Ok(response.path()))
            : rust::Result<std::string>(rust::Err(create_error(status)));
    }

    rust::Result<std::map<std::string, std::string>> InterceptClient::get_environment_update(const std::map<std::string, std::string>& input)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::GetEnvironmentUpdate");

        grpc::ClientContext context;
        supervise::EnvironmentRequest request;
        supervise::EnvironmentResponse response;

        request.mutable_environment()->insert(input.begin(), input.end());

        const grpc::Status status = stub_->GetEnvironmentUpdate(&context, request, &response);
        spdlog::debug("gRPC call finished: {}", status.ok());
        if (status.ok()) {
            std::map<std::string, std::string> copy(response.environment().begin(), response.environment().end());
            return rust::Ok(copy);
        }
        return rust::Err(create_error(status));
    }

    rust::Result<int> InterceptClient::report(const std::list<supervise::Event>& events)
    {
        spdlog::debug("gRPC call requested: supervise::Interceptor::Report");

        grpc::ClientContext context;
        supervise::Empty stats;

        std::unique_ptr<grpc::ClientWriter<supervise::Event> > writer(stub_->Report(&context, &stats));
        for (const auto& event : events) {
            if (!writer->Write(event)) {
                break;
            }
        }
        writer->WritesDone();

        const grpc::Status status = writer->Finish();
        spdlog::debug("gRPC call finished: {}", status.ok());
        return status.ok()
               ? rust::Result<int>(rust::Ok(0))
               : rust::Result<int>(rust::Err(create_error(status)));
    }
}
