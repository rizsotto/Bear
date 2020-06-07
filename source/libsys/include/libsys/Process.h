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

#include <list>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <unistd.h>

namespace sys {

    class ExitStatus {
    public:
        ExitStatus(bool is_code, int code);

        ExitStatus() = delete ;
        ~ExitStatus() noexcept = default;

        std::optional<int> code() const;
        std::optional<int> signal() const;

        bool is_signaled() const;
        bool is_exited() const;

    private:
        bool is_code_;
        int code_;
    };

    class Process {
    public:
        class Builder;

        [[nodiscard]] pid_t get_pid() const;

        rust::Result<ExitStatus> wait(bool request_for_signals = false);
        rust::Result<int> kill(int num);

    public:
        Process() = delete;
        ~Process() = default;

        Process(const Process&) = default;
        Process(Process&&) noexcept = default;

        Process& operator=(const Process&) = default;
        Process& operator=(Process&&) noexcept = default;

    private:
        friend Builder;
        explicit Process(pid_t pid);

        pid_t pid_;
    };

    class Process::Builder {
    public:
        explicit Builder(std::string program);
        explicit Builder(const std::string_view& program);
        ~Builder() = default;

        Builder& add_argument(const char* param);
        Builder& add_argument(std::string&& param);
        Builder& add_argument(const std::string_view& param);

        template <typename InputIt>
        Builder& add_arguments(InputIt first, InputIt last)
        {
            for (InputIt it = first; it != last; ++it) {
                add_argument(*it);
            }
            return *this;
        }

        Builder& set_environment(std::map<std::string, std::string>&&);
        Builder& set_environment(const std::map<std::string, std::string>&);

        // This is hard to implement and not used in this project.
        //Builder& set_working_dir(const std::string&);

        //Builder& set_std_in(int fd);
        //Builder& set_std_out(int fd);
        //Builder& set_std_err(int fd);

        rust::Result<std::string> resolve_executable();
        rust::Result<Process> spawn(bool with_preload);
        // This is hard to implement and not used in this project.
        //rust::Result<std::string> output();
        //rust::Result<int> status();

    public:
        Builder(const Builder&) = default;
        Builder(Builder&&) noexcept = default;

        Builder& operator=(const Builder&) = default;
        Builder& operator=(Builder&&) noexcept = default;

    private:
        std::string program_;
        std::list<std::string> parameters_;
        std::map<std::string, std::string> environment_;
    };
}
