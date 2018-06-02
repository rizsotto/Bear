/*  Copyright (C) 2012-2017 by László Nagy
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

#include "libexec_a/Interface.h"
#include "intercept_a/Session.h"

namespace pear {

//    pear::Result<pear::Session> parse(int argc, char *argv[]) noexcept {
//        pear::Session result;
//
//        int opt;
//        while ((opt = getopt(argc, argv, "t:l:f:s:")) != -1) {
//            switch (opt) {
//                case 't':
//                    result.session.destination = optarg;
//                    break;
//                case 'l':
//                    result.library = optarg;
//                    break;
//                case 'f':
//                    result.execution.file = optarg;
//                    break;
//                case 's':
//                    result.execution.search_path = optarg;
//                    break;
//                default: /* '?' */
//                    return pear::Result<pear::Session>::failure(
//                            std::runtime_error(
//                                    "Usage: intercept [OPTION]... -- command\n\n"
//                                    "  -t <target url>       where to send execution reports\n"
//                                    "  -l <path to libexec>   where to find the ear libray\n"
//                                    "  -f <file>             file parameter\n"
//                                    "  -s <search_path>      search path parameter\n"));
//            }
//        }
//
//        if (optind >= argc) {
//            return pear::Result<pear::Session>::failure(
//                    std::runtime_error(
//                            "Usage: intercept [OPTION]... -- command\n"
//                            "Expected argument after options"));
//        } else {
//            // TODO: do validation!!!
//            result.session.reporter = argv[0];
//            result.execution.command = const_cast<const char **>(argv + optind);
//            return pear::Result<pear::Session>::success(std::move(result));
//        }
//    }

    pear::Result<pear::SessionPtr> Session::parse(int argc, char **argv) noexcept {
        return pear::Result<pear::SessionPtr>::failure(std::runtime_error("placeholder"));
    }

    ::pear::Environment::Builder &
    LibrarySession::set(::pear::Environment::Builder &builder) const noexcept {
        return builder;
    }

    ::pear::Environment::Builder &
    WrapperSession::set(::pear::Environment::Builder &builder) const noexcept {
        return builder;
    }
}