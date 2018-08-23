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

    namespace types {

        template <typename T>
        struct Ok {
            explicit Ok(const T& value)
                    : value_(value)
            { }

            explicit Ok(T&& value) noexcept
                    : value_(value)
            { }

            T value_;
        };

        template <typename E>
        struct Err {
            explicit Err(const E& value)
                    : value_(value)
            { }

            explicit Err(E&& value) noexcept
                    : value_(value)
            { }

            E value_;
        };

    }

    template<typename T, typename CleanT = typename std::decay<T>::type>
    types::Ok<CleanT> Ok(T&& val) {
        return types::Ok<CleanT>(std::forward<T>(val));
    }

    template<typename E, typename CleanE = typename std::decay<E>::type>
    types::Err<CleanE> Err(E&& val) {
        return types::Err<CleanE>(std::forward<E>(val));
    }


    template<typename T, typename E = std::runtime_error>
    class Result {
    public:
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

        Result(types::Ok<T>&& ok) noexcept;  // NOLINT

        Result(types::Err<E>&& err) noexcept;  // NOLINT

    private:
        std::variant<T, E> state_;
    };


    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::map(std::function<U(const T &)> const &f) const noexcept {
        if (std::holds_alternative<T>(state_))
            return Ok(f(std::get<T>(state_)));
        else
            return Err(std::get<E>(state_));
    }

    template<typename T, typename E>
    template<typename U>
    Result<U, E> Result<T, E>::bind(std::function<Result<U, E>(const T &)> const &f) const noexcept {
        if (std::holds_alternative<T>(state_))
            return f(std::get<T>(state_));
        else
            return Err(std::get<E>(state_));
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
    Result<T, E>::Result(types::Ok<T> &&ok) noexcept
            : state_(ok.value_)
    { }

    template<typename T, typename E>
    Result<T, E>::Result(types::Err<E> &&err) noexcept
            : state_(err.value_)
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
