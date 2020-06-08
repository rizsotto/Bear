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

#include "config.h"
#include "libsys/Signal.h"

 #ifdef HAVE_SIGNAL_H
#include <signal.h>
#endif

namespace {

    int SIGNALS_TO_FORWARD[] = {
#ifdef HAVE_SIGNAL
        SIGCONT,
        SIGHUP,
        SIGINFO,
        SIGINT,
        SIGPIPE,
        SIGPOLL,
        SIGQUIT,
        SIGSTOP,
        SIGTERM,
        SIGTSTP,
        SIGTTIN,
        SIGTTOU,
        SIGUSR1,
        SIGUSR2,
        SIGVTALRM,
        SIGWINCH,
        SIGXCPU,
        SIGXFSZ
#endif
    };

    sys::Process* CHILD_PROCESS = nullptr;

    void handler(int signum) {
        if (CHILD_PROCESS != nullptr) {
            CHILD_PROCESS->kill(signum);
        }
    }

}

namespace sys {

    [[maybe_unused]] [[maybe_unused]] SignalForwarder::SignalForwarder(Process* child)
    {
        CHILD_PROCESS = child;
#ifdef HAVE_SIGNAL
        for (auto signum : SIGNALS_TO_FORWARD) {
            signal(signum, &handler);
        }
#endif
    }

    SignalForwarder::~SignalForwarder()
    {
        CHILD_PROCESS = nullptr;
    }
}