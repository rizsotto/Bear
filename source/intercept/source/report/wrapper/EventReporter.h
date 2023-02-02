/*  Copyright (C) 2012-2023 by László Nagy
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

#include "Domain.h"
#include "report/wrapper/EventFactory.h"
#include "report/wrapper/RpcClients.h"
#include "libresult/Result.h"
#include "libsys/Process.h"

namespace wr {

    /**
     * Reports events to the interceptor gRPC service.
     *
     * Depend on the implementation, it can collect the events and send at the very
     * end, or it can send it immediately (sync or async).
     */
    class EventReporter {
    public:
        explicit EventReporter(const wr::SessionLocator& session_locator) noexcept;
        ~EventReporter() noexcept = default;

        void report_start(ProcessId pid, const Execution &execution);
        void report_wait(sys::ExitStatus exit_status);

        NON_DEFAULT_CONSTRUCTABLE(EventReporter)
        NON_COPYABLE_NOR_MOVABLE(EventReporter)

    private:
        EventFactory event_factory;
        InterceptorClient client;
    };
}
