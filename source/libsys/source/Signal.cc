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

#include "libsys/Signal.h"
#include "libsys/Process.h"

#include <set>

namespace {

    inline
    constexpr bool shall_forward(int signum) {
        switch (signum) {
            case SIGKILL:
            case SIGCHLD:
                return false;
            default:
                return true;
        }
    }

    std::set<pid_t> CHILD_PROCESSES;

    void handler(int signum) {
        if (shall_forward(signum)) {
            for (auto pid : CHILD_PROCESSES) {
                ::kill(pid, signum);
            }
        }
    }
}

namespace sys {

    SignalForwarder::SignalForwarder(const Process &child) noexcept
            : pid_(child.get_pid())
            , handlers_()
    {
        CHILD_PROCESSES.insert(pid_);
        for (int signum = 1; signum < NSIG; ++signum) {
            handlers_[signum] = ::signal(signum, &handler);
        }
    }

    SignalForwarder::~SignalForwarder() noexcept
    {
        CHILD_PROCESSES.erase(pid_);
        for (int signum = 1; signum < NSIG; ++signum) {
            ::signal(signum, handlers_[signum]);
        }
    }
}
