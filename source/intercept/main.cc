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

// How it should works?
//
// - Choose interception mode (wrapper or preload)
//   - Set up environment variables accordingly
// - Create communication channel for `er` to report process execution
//   - Listens to the channel and collect the received reports
// - Calls `er` to supervise the build (with the received command).
//   - Wait until the child process terminates. (store exit code)
// - Close communication channel.
// - Writes output.
// - Return child exit code.
//
// Communication channel means: filesystem or socket. Do migration easy,
// start with the filesystem (means create a temporary directory and
// delete it when everything is finished). This can be later changed to
// UNIX or TCP sockets.

int main() {
    return 0;
}
