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

#include "config.h"
#include "libresult/Result.h"

#include <filesystem>
#include <list>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <unistd.h>

namespace fs = std::filesystem;

namespace sys {

    class ExitStatus {
    public:
        ExitStatus(bool is_code, int code);

        ExitStatus() = delete ;
        ~ExitStatus() noexcept = default;

        [[nodiscard]]
        std::optional<int> code() const;
        [[nodiscard]]
        std::optional<int> signal() const;

        [[nodiscard]]
        bool is_signaled() const;
        [[nodiscard]]
        bool is_exited() const;

    private:
        bool is_code_;
        int code_;
    };

    class Process {
    public:
        class Builder;

        [[nodiscard]]
        pid_t get_pid() const;

        rust::Result<ExitStatus> wait(bool request_for_signals = false);
        rust::Result<int> kill(int num);

    public:
        explicit Process(pid_t pid);

        Process() = delete;
        ~Process() = default;

        Process(const Process&) = default;
        Process(Process&&) noexcept = default;

        Process& operator=(const Process&) = default;
        Process& operator=(Process&&) noexcept = default;

    private:
        pid_t pid_;
    };

    class Process::Builder {
    public:
        explicit Builder(fs::path program);
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

        // These are not used in this project.
        //Builder& set_working_dir(const std::string&);

        //Builder& set_std_in(int fd);
        //Builder& set_std_out(int fd);
        //Builder& set_std_err(int fd);

        rust::Result<fs::path> resolve_executable();

        rust::Result<Process> spawn();

#ifdef SUPPORT_PRELOAD
        rust::Result<Process> spawn_with_preload();
#endif

    public:
        Builder(const Builder&) = default;
        Builder(Builder&&) noexcept = default;

        Builder& operator=(const Builder&) = default;
        Builder& operator=(Builder&&) noexcept = default;

    private:
        fs::path program_;
        std::list<std::string> parameters_;
        std::map<std::string, std::string> environment_;
    };
}
