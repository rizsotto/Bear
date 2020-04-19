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

#include "Interceptor.h"
#include "Reporter.h"
#include "Session.h"

namespace ic {

    InterceptorImpl::InterceptorImpl(Reporter& reporter, const Session& session)
            : ::supervise::Interceptor::Service()
            , reporter_(reporter)
            , session_(session)
            , lock()
    {
    }

    ::grpc::Status InterceptorImpl::GetWrappedCommand(::grpc::ServerContext* context, const ::supervise::WrapperRequest* request, ::supervise::WrapperResponse* response)
    {
        std::lock_guard<std::mutex> guard(lock);

        return session_.resolve(request->name())
                .map<grpc::Status>([&response](auto path) {
                    response->set_path(path.data());
                    return ::grpc::Status::OK;
                })
                .unwrap_or(grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "not recognized wrapper"));
    }

    ::grpc::Status InterceptorImpl::GetEnvironmentUpdate(::grpc::ServerContext* context, const ::supervise::EnvironmentRequest* request, ::supervise::EnvironmentResponse* response)
    {
        std::lock_guard<std::mutex> guard(lock);

        const std::map<std::string, std::string> copy(request->environment().begin(), request->environment().end());
        return session_.update(copy)
            .map<grpc::Status>([&response](auto update) {
                response->mutable_environment()->insert(update.begin(), update.end());
                return grpc::Status::OK;
            })
            .unwrap_or(grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "environment update failed"));
    }

    ::grpc::Status InterceptorImpl::Report(::grpc::ServerContext* context, ::grpc::ServerReader<::supervise::Event>* reader, ::supervise::Empty* response)
    {
        std::lock_guard<std::mutex> guard(lock);

        Execution::Builder builder;

        ::supervise::Event event;
        while (reader->Read(&event)) {
            builder.add(event);
        }
        reporter_.report(builder.build());

        return ::grpc::Status::OK;
    }
}
