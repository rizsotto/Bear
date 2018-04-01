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

#pragma once

#include <functional>

namespace ear {

    constexpr char library_flag[] = "-l";
    constexpr char destination_flag[] = "-t";
    constexpr char verbose_flag[] = "-v";
    constexpr char file_flag[] = "-f";
    constexpr char search_flag[] = "-s";

    class Serializable {
    public:
        virtual ~Serializable() noexcept = default;

        virtual size_t estimate() const noexcept = 0;

        virtual const char **copy(const char **dst) const noexcept = 0;

    public:
        Serializable() noexcept = default;

        Serializable(Serializable const &) = delete;

        Serializable(Serializable &&) noexcept = delete;

        Serializable &operator=(Serializable const &) = delete;

        Serializable &operator=(Serializable &&) noexcept = delete;
    };

    class Serializer {
    public:
        explicit Serializer(Serializable const &) noexcept;

        int forward(std::function<int (const char **)>) const noexcept;

    public:
        Serializer() noexcept = delete;

        Serializer(Serializer const &) = delete;

        Serializer(Serializer &&) noexcept = delete;

        ~Serializer() noexcept = default;

        Serializer &operator=(Serializer const &) = delete;

        Serializer &operator=(Serializer &&) noexcept = delete;
    };

    struct Report : public Serializable {
        struct Library : public Serializable {
            const char *reporter;
            const char *library;
            const char *destination;
            bool verbose;

            size_t estimate() const noexcept override;
            const char **copy(const char **dst) const noexcept override;
        };
        struct Execution : public Serializable {
            const char **command;
            const char *file;
            const char *search_path;

            size_t estimate() const noexcept override;
            const char **copy(const char **dst) const noexcept override;
        };

        Library library;
        Execution execution;

        size_t estimate() const noexcept override;
        const char **copy(const char **dst) const noexcept override;
    };

}
