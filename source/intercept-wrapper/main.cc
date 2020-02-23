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
// - The `wrapper` shall be a single executable with soft links created to it
//   with the name of the wrapped command. (`cc`, `c++`, `ar`, `ld`, `as`, etc...)
//
// - When it's executed: it figures out what commands it wraps.
//   (The name comes from the argument, get the `basename` of it, and that's it)
// - Look up what is the real executable for that command (full path)
//   (This can be a file what the the `intercept` plants for the session.
//   The location of that file might come from an environment variable.
//   The `wrapper` needs to read that file and find the path to the wrapped command.)
// - Calls `er`, pass the real executable and the arguments itself received.
//   (calls mean `execve`)  `er` will report the call and supervise the process.


int main(int argc, char* argv[], char* envp[])
{
    return 0;
}
