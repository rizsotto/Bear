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

#include "libsys/Signal.h"
#include "libsys/Process.h"

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

    sys::Process* CHILD_PROCESS = nullptr;

    void handler(int signum) {
        if (CHILD_PROCESS != nullptr && shall_forward(signum)) {
            CHILD_PROCESS->kill(signum);
        }
    }
}

namespace sys {

    SignalForwarder::SignalForwarder(Process& child)
            : handlers_()
    {
        CHILD_PROCESS = &child;
        for (int signum = 1; signum < NSIG; ++signum) {
            handlers_[signum] = ::signal(signum, &handler);
        }
    }

    SignalForwarder::~SignalForwarder() noexcept
    {
        CHILD_PROCESS = nullptr;
        for (int signum = 1; signum < NSIG; ++signum) {
            ::signal(signum, handlers_[signum]);
        }
    }
}
