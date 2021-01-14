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
#include "intercept/Flags.h"
#include "collect/Reporter.h"
#include "libsys/Os.h"

#include <spdlog/spdlog.h>

#include <fstream>
#include <memory>
#include <utility>
#include <unistd.h>

namespace {

    using HostInfo = std::map<std::string, std::string>;

    rust::Result<HostInfo> create_host_info()
    {
        return sys::os::get_uname()
#ifdef HAVE_CS_PATH
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_PATH)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_PATH", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_GNU_LIBC_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBC_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
#ifdef HAVE_CS_GNU_LIBPTHREAD_VERSION
            .map<HostInfo>([](auto result) {
                sys::os::get_confstr(_CS_GNU_LIBPTHREAD_VERSION)
                    .map<int>([&result](auto value) {
                        result.insert({ "_CS_GNU_LIBPTHREAD_VERSION", value });
                        return 0;
                    });
                return result;
            })
#endif
            .map_err<std::runtime_error>([](auto error) {
                return std::runtime_error(fmt::format("failed to get host info: {}", error.what()));
            });
    }
}

namespace ic {

    rust::Result<Reporter::Ptr> Reporter::from(const flags::Arguments& flags)
    {
        auto host_info = create_host_info();
        auto output = flags.as_string(OUTPUT);
        auto events = output
                .and_then<EventsDatabase::Ptr>([](auto file) {
                    return EventsDatabase::create(file);
                });

        return merge(host_info, output, events)
                .map<Reporter::Ptr>([](auto tuple) {
                    const auto&[host_info, output, events] = tuple;
                    return std::make_shared<Reporter>(fs::path(output), events);
                });
    }

    Reporter::Reporter(fs::path output,
                       ic::EventsDatabase::Ptr events)
            : output_(std::move(output))
            , events_(std::move(events))
    { }

    void Reporter::report(const rpc::Event& event)
    {
        events_->insert_event(event)
                .on_error([](auto error) {
                    spdlog::warn("Writing event into database failed: {} Ignored.", error.what());
                });
    }
}
