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

#include "collect/Reporter.h"
#include "collect/Services.h"
#include "collect/Session.h"

namespace ic {

    SupervisorImpl::SupervisorImpl(const Session& session)
        : rpc::Supervisor::Service()
        , session_(session)
    {
    }

    ::grpc::Status SupervisorImpl::Update(::grpc::ServerContext*, const rpc::Environment* request, rpc::Environment* response)
    {
        const std::map<std::string, std::string> copy(request->values().begin(), request->values().end());
        return session_.update(copy)
            .map<grpc::Status>([&response](auto update) {
                response->mutable_values()->insert(update.begin(), update.end());
                return grpc::Status::OK;
            })
            .unwrap_or(grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "environment update failed"));
    }

    ::grpc::Status SupervisorImpl::ResolveProgram(::grpc::ServerContext*, const rpc::ResolveRequest* request, rpc::ResolveResponse* response)
    {
        return session_.resolve(request->path())
            .map<grpc::Status>([&response](auto path) {
                response->set_path(path.data());
                return ::grpc::Status::OK;
            })
            .unwrap_or(grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "not recognized wrapper"));
    }

    InterceptorImpl::InterceptorImpl(Reporter& reporter)
        : rpc::Interceptor::Service()
        , reporter_(reporter)
        , lock_()
    {
    }

    ::grpc::Status InterceptorImpl::Register(::grpc::ServerContext*, const rpc::Event* request, rpc::Empty*)
    {
        std::lock_guard<std::mutex> guard(lock_);

        reporter_.report(*request);

        return ::grpc::Status::OK;
    }
}
