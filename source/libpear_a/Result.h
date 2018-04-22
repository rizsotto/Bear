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

#include <variant>
#include <functional>


namespace ear {

    // TODO: make it work with void too!
    template<typename T, typename E>
    class Result {
    public:
        static Result success(T &&value) noexcept;

        static Result failure(const E &value) noexcept;

        template<typename U>
        Result<U, E> map(std::function<U(T &)> const &f) noexcept;

        template<typename U>
        Result<U, E> bind(std::function<Result<U, E>(T &)> const &f) noexcept;

        T get_or_else(const T &value) const noexcept;

        Result<T, E> const &handle_with(std::function<void(const E &)> const &f) const noexcept;

    public:
        Result(Result &&other) noexcept;

        Result &operator=(Result &&other) noexcept;

        ~Result() noexcept = default;

    public:
        Result() = delete;

        Result(const Result &other) noexcept = delete;

        Result &operator=(const Result &other) = delete;

    private:
        std::variant<T, E> state_;

        explicit Result(T &&other) noexcept;

        explicit Result(E const &error) noexcept;
    };


    template<typename T, typename E>
    Result<T, E>::Result(const E &error) noexcept
            : state_(error) {}

    template<typename T, typename E>
    Result<T, E>::Result(Result &&other) noexcept
            : state_(std::move(other.state_)) {}

    template<typename T, typename E>
    Result<T, E>::Result(T &&other) noexcept
            : state_(std::move(other)) {}

    template<typename T, typename E>
    Result<T, E> &Result<T, E>::operator=(Result &&other) noexcept {
        if (this != &other) {
            state_ = other.state_;
        }
        return *this;
    }

    template<typename T, typename E>
    Result<T, E> Result<T, E>::success(T &&value) noexcept {
        return Result(std::move(value));
    }

    template<typename T, typename E>
    Result<T, E> Result<T, E>::failure(const E &value) noexcept {
        return Result(value);
    }

    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::map(std::function<U(T &)> const &f) noexcept {
        if (auto ptr = std::get_if<T>(&state_)) {
            return Result<U, E>::success(std::move(f(*ptr)));
        } else if (auto error = std::get_if<E>(&state_)) {
            return Result<U, E>::failure(*error);
        }
    }

    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::bind(std::function<Result<U, E>(T &)> const &f) noexcept {
        if (auto ptr = std::get_if<T>(&state_)) {
            return f(*ptr);
        } else if (auto error = std::get_if<E>(&state_)) {
            return Result<U, E>::failure(*error);
        }
    }

    template<typename T, typename E>
    T Result<T, E>::get_or_else(const T &value) const noexcept {
        if (auto ptr = std::get_if<T>(&state_)) {
            return *ptr;
        } else {
            return value;
        }
    }

    template<typename T, typename E>
    Result<T, E> const &Result<T, E>::handle_with(std::function<void(const E &)> const &f) const noexcept {
        if (auto error = std::get_if<E>(&state_)) {
            f(*error);
        };
        return *this;
    }
}

namespace pear {

    template <typename T>
    using Result = ::ear::Result<T, std::runtime_error>;

}
