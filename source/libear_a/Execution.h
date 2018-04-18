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

#include "libear_a/State.h"
#include "libear_a/SessionSerializer.h"
#include "libear_a/DynamicLinker.h"

namespace ear {

    constexpr char command_separator[] = "--";
    constexpr char file_flag[] = "-f";
    constexpr char search_flag[] = "-s";

    class Execution {
    public:
        int apply(DynamicLinker const &linker, State const *state) noexcept {
            return (state == nullptr)
                ? this->apply(linker)
                : this->apply(linker, LibrarySessionSerializer(state->get_input()));
        }

    protected:
        virtual int apply(DynamicLinker const &) noexcept = 0;
        virtual int apply(DynamicLinker const &, Serializable const &) noexcept = 0;

        // TODO: use result type for this method (to do proper error handling).
        int forward(Serializable const &session,
                    Serializable const &execution,
                    std::function<int (const char *, const char **)> const &function) noexcept {
            size_t const size = session.estimate() + execution.estimate();
            char const *dst[size];
            char const **it = dst;
            char const **const end = it + size;

            it = session.copy(it, end);
            it = execution.copy(it, end);

            return function(dst[0], dst);
        }

        class Serializer;
    };

    class Execution::Serializer : public Serializable {
    public:
        using Estimator = std::function<size_t ()>;
        using Copier = std::function<char const ** (char const **, char const **)>;

    public:
        Serializer(Estimator const &estimator, Copier const &copier) noexcept
                : estimator_(estimator)
                , copier_(copier)
        { }

        size_t estimate() const noexcept override {
            return estimator_();
        }

        const char **copy(char const **begin, char const **end) const noexcept override {
            return copier_(begin, end);
        }

    private:
        Estimator const &estimator_;
        Copier const &copier_;
    };


    class Execve : public Execution {
    public:
        Execve(const char *path, char *const argv[], char *const envp[]) noexcept
                : path_(path)
                , argv_(const_cast<const char **>(argv))
                , envp_(const_cast<const char **>(envp))
        { }

        int apply(DynamicLinker const &linker) noexcept override {
            auto fp = linker.execve();
            return fp(path_, const_cast<char *const *>(argv_), const_cast<char *const *>(envp_));
        }

        int apply(DynamicLinker const &linker, Serializable const &session) noexcept override {
            Execution::Serializer serializer(
                    [this]() {
                        return ::ear::array::length(argv_) + 2;
                    },
                    [this](auto begin, auto end) {
                        const char **argv_begin = argv_;
                        const char **argv_end = argv_begin + ::ear::array::length(argv_);

                        auto it = begin;
                        *it++ = command_separator;
                        return ::ear::array::copy(argv_begin, argv_end, it, end);
                    }
            );

            auto fp = linker.execve();
            return forward(session, serializer,
                           [this, &fp](auto cmd, auto args) {
                               return fp(cmd,
                                         const_cast<char *const *>(args),
                                         const_cast<char *const *>(envp_));
                           }
            );
        }

    private:
        const char *path_;
        const char **argv_;
        const char **envp_;
    };

}
