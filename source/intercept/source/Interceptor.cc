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
    {
    }

    ::grpc::Status InterceptorImpl::GetWrappedCommand(::grpc::ServerContext* context, const ::supervise::WrapperRequest* request, ::supervise::WrapperResponse* response)
    {
        const char* path = session_.resolve(request->name());
        if (path != nullptr) {
            response->set_path(path);
            return ::grpc::Status::OK;
        }
        return ::grpc::Status(::grpc::StatusCode::INVALID_ARGUMENT, "not recognized wrapper");
    }

    ::grpc::Status InterceptorImpl::GetEnvironmentUpdate(::grpc::ServerContext* context, const ::supervise::Empty* request, ::supervise::EnvironmentUpdate* response)
    {
        auto appends = session_.appends();
        response->mutable_appends()->insert(appends.begin(), appends.end());
        auto overrides = session_.overrides();
        response->mutable_overrides()->insert(overrides.begin(), overrides.end());
        return ::grpc::Status::OK;
    }

    ::grpc::Status InterceptorImpl::Report(::grpc::ServerContext* context, ::grpc::ServerReader<::supervise::Event>* reader, ::supervise::Empty* response)
    {
        Execution::Builder builder;

        ::supervise::Event event;
        while (reader->Read(&event)) {
            builder.add(event);
        }
        reporter_.report(builder.build());

        return ::grpc::Status::OK;
    }
}
