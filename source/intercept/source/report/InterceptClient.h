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

#include "libresult/Result.h"

#include <grpcpp/channel.h>
#include "supervise.grpc.pb.h"

#include <list>

namespace rpc {

    struct Session {
        const std::string destination;
    };

    class InterceptClient {
    public:
        explicit InterceptClient(const Session& session);

        InterceptClient() = delete;
        InterceptClient(const InterceptClient&) = delete;
        InterceptClient(InterceptClient&&) noexcept = delete;

        InterceptClient& operator=(const InterceptClient&) = delete;
        InterceptClient& operator=(InterceptClient&&) noexcept = delete;

    public:
        rust::Result<std::string> get_wrapped_command(const std::string&);
        rust::Result<std::map<std::string, std::string>> get_environment_update(const std::map<std::string, std::string>&);
        rust::Result<int> report(supervise::Event&&);

    private:
        std::shared_ptr<::grpc::Channel> channel_;
        std::unique_ptr<supervise::Supervisor::Stub> supervisor_;
        std::unique_ptr<supervise::Interceptor::Stub> interceptor_;
    };
}