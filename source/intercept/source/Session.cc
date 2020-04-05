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

#include "Session.h"

#include "config.h"

#ifdef HAVE_SYS_UTSNAME_H
#include <sys/utsname.h>
#endif
#include <unistd.h>

namespace {

#ifdef HAVE_CONFSTR
    rust::Result<std::string> get_confstr(const int key)
    {
        if (const size_t buffer_size = confstr(key, nullptr, 0); buffer_size != 0) {
            char buffer[buffer_size];
            if (const size_t size = confstr(key, buffer, buffer_size); size != 0) {
                return rust::Ok(std::string(buffer));
            }
        }
        return rust::Err(std::runtime_error("confstr failed."));
    }
#endif

    rust::Result<ic::Session::HostInfo> create_host_info()
    {
        std::map<std::string, std::string> result;
#ifdef HAVE_UNAME
        utsname name;
        if (const int status = uname(&name); status >= 0) {
            result.insert({ "sysname", std::string(name.sysname) });
            result.insert({ "release", std::string(name.release) });
            result.insert({ "version", std::string(name.version) });
            result.insert({ "machine", std::string(name.machine) });
        }
#endif
#ifdef HAVE_CONFSTR
#ifdef HAVE_CS_PATH
        get_confstr(_CS_PATH)
            .map<int>([&result](auto value) {
                result.insert({ "_CS_PATH", value });
                return 0;
            });
#endif
#ifdef HAVE_CS_GNU_LIBC_VERSION
        get_confstr(_CS_GNU_LIBC_VERSION)
            .map<int>([&result](auto value) {
                result.insert({ "_CS_GNU_LIBC_VERSION", value });
                return 0;
            });
#endif
#ifdef HAVE__CS_GNU_LIBPTHREAD_VERSION
        get_confstr(__CS_GNU_LIBPTHREAD_VERSION)
            .map<int>([&result](auto value) {
                result.insert({ "__CS_GNU_LIBPTHREAD_VERSION", value });
                return 0;
            });
#endif
#endif
        return (result.empty())
            ? rust::Result<ic::Session::HostInfo>(rust::Err(std::runtime_error("not implemented")))
            : rust::Result<ic::Session::HostInfo>(rust::Ok(result));
    }

    class LibraryPreloadSession : public ic::Session {
    public:
        explicit LibraryPreloadSession(HostInfo &&host_info);

    public:
        [[nodiscard]] rust::Result<std::string_view> resolve(const std::string& name) const override;
        [[nodiscard]] rust::Result<std::map<std::string, std::string>> update(const std::map<std::string, std::string>& env) const override;
        [[nodiscard]] rust::Result<int> supervise(const std::vector<std::string_view>& command) const override;

        void set_server_address(const std::string&) override;

        [[nodiscard]] const HostInfo& get_host_info() const override;
        [[nodiscard]] std::string get_session_type() const override;

    private:
        HostInfo host_info_;
        std::string server_address_;
    };

    LibraryPreloadSession::LibraryPreloadSession(ic::Session::HostInfo&& host_info)
            : host_info_(host_info)
            , server_address_()
    {
    }

    rust::Result<std::string_view> LibraryPreloadSession::resolve(const std::string& name) const
    {
        return rust::Err(std::runtime_error("The session does not support resolve."));
    }

    rust::Result<std::map<std::string, std::string>> LibraryPreloadSession::update(const std::map<std::string, std::string>& env) const
    {
        // TODO
        return rust::Ok(env);
    }

    rust::Result<int> LibraryPreloadSession::supervise(const std::vector<std::string_view>& command) const
    {
        // TODO
        return rust::Ok(0);
    }

    void LibraryPreloadSession::set_server_address(const std::string& value)
    {
        server_address_ = value;
    }

    const ic::Session::HostInfo& LibraryPreloadSession::get_host_info() const
    {
        return host_info_;
    }

    std::string LibraryPreloadSession::get_session_type() const
    {
        return std::string("library_preload");
    }
}

namespace ic {

    rust::Result<Session::SharedPtr> Session::from(const flags::Arguments& args)
    {
        return create_host_info()
                .map<Session::SharedPtr>([](auto info) {
                    return std::shared_ptr<Session>(new LibraryPreloadSession(std::move(info)));
                });
    }
}
