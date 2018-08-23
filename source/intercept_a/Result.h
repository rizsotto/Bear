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
#include <stdexcept>

namespace pear {

    template<typename T, typename E = std::runtime_error>
    class Result {
    public:
        static Result success(T &&value) noexcept;

        static Result success(const T &value) noexcept;

        static Result failure(E &&value) noexcept;

        static Result failure(const E &value) noexcept;

        template<typename U>
        Result<U, E> map(std::function<U(const T &)> const &f) const noexcept;

        template<typename U>
        Result<U, E> bind(std::function<Result<U, E>(const T &)> const &f) const noexcept;

        const T &get_or_else(const T &value) const noexcept;

        Result<T, E> const & handle_with(std::function<void(const E &)> const &f) const noexcept;

    public:
        ~Result() noexcept = default;

        Result() = delete;

        Result(Result &&other) noexcept = default;

        Result(const Result &other) = delete;

        Result &operator=(Result &&other) noexcept = default;

        Result &operator=(const Result &other) = delete;

    private:
        explicit Result(T &&other) noexcept;

        explicit Result(const T &other) noexcept;

        explicit Result(E &&error) noexcept;

        explicit Result(const E &error) noexcept;

    private:
        std::variant<T, E> state_;
    };


    template<typename T, typename E>
    Result<T, E> Result<T, E>::success(T &&value) noexcept {
        return Result(std::move(value));
    }

    template<typename T, typename E>
    Result<T, E> Result<T, E>::success(const T &value) noexcept {
        return Result(value);
    }

    template<typename T, typename E>
    Result<T, E> Result<T, E>::failure(E &&value) noexcept {
        return Result(std::move(value));
    }

    template<typename T, typename E>
    Result<T, E> Result<T, E>::failure(const E &value) noexcept {
        return Result(value);
    }

    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::map(std::function<U(const T &)> const &f) const noexcept {
        return (std::holds_alternative<T>(state_))
            ? Result<U, E>::success(f(std::get<T>(state_)))
            : Result<U, E>::failure(std::get<E>(state_));
    }

    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::bind(std::function<Result<U, E>(const T &)> const &f) const noexcept {
        return (std::holds_alternative<T>(state_))
            ? f(std::get<T>(state_))
            : Result<U, E>::failure(std::get<E>(state_));
    }

    template<typename T, typename E>
    const T &Result<T, E>::get_or_else(const T &value) const noexcept {
        return (std::holds_alternative<T>(state_))
            ? std::get<T>(state_)
            : value;
    }

    template<typename T, typename E>
    Result<T, E> const & Result<T, E>::handle_with(std::function<void(const E &)> const &f) const noexcept {
        if (auto error = std::get_if<E>(&state_)) {
            f(*error);
        };
        return *this;
    }

    template<typename T, typename E>
    Result<T, E>::Result(T &&other) noexcept
            : state_(std::move(other))
    { }

    template<typename T, typename E>
    Result<T, E>::Result(const T &other) noexcept
            : state_(other)
    { }

    template<typename T, typename E>
    Result<T, E>::Result(E &&error) noexcept
            : state_(std::move(error))
    { }

    template<typename T, typename E>
    Result<T, E>::Result(const E &error) noexcept
            : state_(error)
    { }


    template <typename T1, typename T2>
    Result<std::tuple<T1, T2>> merge(const Result<T1> &t1, const Result<T2> &t2) {
        return t1.template bind<std::tuple<T1, T2>>([&t2](auto &t1_value) {
            return t2.template map<std::tuple<T1, T2>>([&t1_value](auto &t2_value) {
                return std::make_tuple(t1_value, t2_value);
            });
        });
    }

    template <typename T1, typename T2, typename T3>
    Result<std::tuple<T1, T2, T3>> merge(const Result<T1> &t1, const Result<T2> &t2, const Result<T3> &t3) {
        return t1.template bind<std::tuple<T1, T2, T3>>([&t2, &t3](auto &t1_value) {
            return t2.template bind<std::tuple<T1, T2, T3>>([&t1_value, &t3](auto &t2_value) {
                return t3.template map<std::tuple<T1, T2, T3>>([&t1_value, &t2_value](auto &t3_value) {
                    return std::make_tuple(t1_value, t2_value, t3_value);
                });
            });
        });
    }

}
