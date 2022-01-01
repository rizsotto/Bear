/*  Copyright (C) 2012-2022 by László Nagy
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
#include <stdexcept>
#include <type_traits>
#include <cstring>

namespace rust {

    namespace types {

        template <typename T>
        struct Ok {
            explicit Ok(const T& value)
                    : value_(value)
            {
            }

            explicit Ok(T&& value) noexcept
                    : value_(std::move(value))
            {
            }

            T value_;
        };

        template <typename E>
        struct Err {
            explicit Err(const E& value)
                    : value_(value)
            {
            }

            explicit Err(E&& value) noexcept
                    : value_(value)
            {
            }

            E value_;
        };
    }

    // Most of the internal is about to implement a storage for the values.
    //
    // This can be done with `std::variant` which is available in C++17.
    // To make this code more portable the implementation is using C++14
    // language constructs only.
    namespace internals {

        template <typename T, typename E>
        struct Storage {
            static constexpr size_t Size = sizeof(T) > sizeof(E) ? sizeof(T) : sizeof(E);
            static constexpr size_t Align = sizeof(T) > sizeof(E) ? alignof(T) : alignof(E);

            typedef typename std::aligned_storage<Size, Align>::type type;

            Storage()
                    : initialized_(false)
            {
            }

            void construct(types::Ok<T> ok)
            {
                new (&storage_) T(std::move(ok.value_));
                initialized_ = true;
            }

            void construct(types::Err<E> err)
            {
                new (&storage_) E(std::move(err.value_));
                initialized_ = true;
            }

            template <typename U>
            void raw_construct(U&& value)
            {
                typedef typename std::decay<U>::type CleanU;

                new (&storage_) CleanU(std::forward<U>(value));
                initialized_ = true;
            }

            template <typename U>
            const U& get() const
            {
                return *reinterpret_cast<const U*>(&storage_);
            }

            template <typename U>
            U& get()
            {
                return *reinterpret_cast<U*>(&storage_);
            }

            void destroy_ok()
            {
                if (initialized_) {
                    get<T>().~T();
                    initialized_ = false;
                }
            }

            void destroy_err()
            {
                if (initialized_) {
                    get<E>().~E();
                    initialized_ = false;
                }
            }

            type storage_;
            bool initialized_;
        };
    }

    // Util methods which help to create `Result` types easier.
    template <typename T, typename CleanT = typename std::decay<T>::type>
    types::Ok<CleanT> Ok(T&& value)
    {
        return types::Ok<CleanT>(std::forward<T>(value));
    }

    template <typename E, typename CleanE = typename std::decay<E>::type>
    types::Err<CleanE> Err(E&& value)
    {
        return types::Err<CleanE>(std::forward<E>(value));
    }

    // This class represent a result of a computation.
    //
    // It's planned to implement such construct in later C++ language dialects.
    // That is referred as `std::expected` in proposals.
    //
    //   http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2017/p0323r3.pdf
    //
    // This implementation is more close to the rust language `std::result::Result`
    // type. Where the public functions are following the namings of the rust
    // implementation.
    //
    // The public interface is also trimmed down. The main motivation was:
    //
    // * remove the access methods `ok()` or `err()` methods.
    //   (std::optional in C++17 only)
    // * remove the access methods `unwrap()` or `expect(...)` methods.
    //   (no exception, would be hard to signal wrong access.)
    //
    // Contrast to the C++ std::expected, this type is encourage to use
    // higher order functions (monadic methods) to use the results.
    //
    //   https://doc.rust-lang.org/std/result/enum.Result.html
    template <typename T, typename E = std::runtime_error>
    class Result {
    public:
        Result() = delete;
        ~Result();

        Result(Result&& other) noexcept;
        Result(const Result& other);

        Result& operator=(Result&& other) noexcept;
        Result& operator=(const Result& other);

        Result(types::Ok<T>&& ok) noexcept;
        Result(types::Err<E>&& err) noexcept;

    public:
        [[nodiscard]] bool is_ok() const;
        [[nodiscard]] bool is_err() const;

        template <typename U>
        Result<U, E> map(std::function<U(const T&)> const& f) const;

        template <typename U>
        Result<U, E> map_or(U&& value, std::function<U(const T&)> const& func) const;

        template <typename U>
        Result<U, E> map_or_else(std::function<U(const E&)> const& provider, std::function<U(const T&)> const& f) const;

        template <typename F>
        Result<T, F> map_err(std::function<F(const E&)> const& f) const;

        template <typename U>
        Result<U, E> and_(const Result<U, E>& rhs) const;

        template <typename U>
        Result<U, E> and_then(std::function<Result<U, E>(const T&)> const& f) const;

        Result<T, E> or_(const Result<T, E>& rhs) const;

        Result<T, E> or_else(std::function<Result<T, E>(const E&)> const& f) const;

        const T& unwrap() const;
        const E& unwrap_err() const;
        const T& unwrap_or(const T& value) const;

        T unwrap_or_else(std::function<T(const E&)> const& provider) const;

        const Result<T, E>& on_success(std::function<void(const T&)> const& f) const;
        const Result<T, E>& on_error(std::function<void(const E&)> const& f) const;

    private:
        bool ok_;
        internals::Storage<T, E> storage_;
    };

    template <typename T>
    bool operator==(Result<T, std::runtime_error> const &lhs, Result<T, std::runtime_error> const &rhs) {
        return  (lhs.is_ok() && rhs.is_ok() && (lhs.unwrap() == rhs.unwrap())) ||
                (lhs.is_err() && rhs.is_err() && (std::strcmp(lhs.unwrap_err().what(), rhs.unwrap_err().what()) == 0));
    }

    template <typename T, typename E>
    bool operator==(Result<T, E> const &lhs, Result<T, E> const &rhs) {
        return  (lhs.is_ok() && rhs.is_ok() && (lhs.unwrap() == rhs.unwrap())) ||
                (lhs.is_err() && rhs.is_err() && (lhs.unwrap_err() == rhs.unwrap_err()));
    }

    template <typename T1, typename T2>
    Result<std::tuple<T1, T2>> merge(const Result<T1>& t1, const Result<T2>& t2)
    {
        return t1.template and_then<std::tuple<T1, T2>>([&t2](auto& t1_value) {
            return t2.template map<std::tuple<T1, T2>>([&t1_value](auto& t2_value) {
                return std::make_tuple(t1_value, t2_value);
            });
        });
    }

    template <typename T1, typename T2, typename T3>
    Result<std::tuple<T1, T2, T3>> merge(const Result<T1>& t1, const Result<T2>& t2, const Result<T3>& t3)
    {
        return t1.template and_then<std::tuple<T1, T2, T3>>([&t2, &t3](auto& t1_value) {
            return t2.template and_then<std::tuple<T1, T2, T3>>([&t1_value, &t3](auto& t2_value) {
                return t3.template map<std::tuple<T1, T2, T3>>([&t1_value, &t2_value](auto& t3_value) {
                    return std::make_tuple(t1_value, t2_value, t3_value);
                });
            });
        });
    }

    template<typename T1, typename T2, typename T3, typename T4>
    Result<std::tuple<T1, T2, T3, T4>>
    merge(const Result<T1> &t1, const Result<T2> &t2, const Result<T3> &t3, const Result<T4> &t4) {
        return merge(merge(t1, t2), merge(t3, t4))
                .template map<std::tuple<T1, T2, T3, T4>>([](auto tuple) {
                    const auto&[t12, t34] = tuple;
                    const auto&[t1, t2] = t12;
                    const auto&[t3, t4] = t34;
                    return std::make_tuple(t1, t2, t3, t4);
                });
    }

    template <typename T, typename E>
    Result<T, E>::~Result()
    {
        if (ok_) {
            storage_.destroy_ok();
        } else {
            storage_.destroy_err();
        }
    }

    template <typename T, typename E>
    Result<T, E>::Result(Result<T, E>&& other) noexcept
            : ok_(other.ok_)
            , storage_()
    {
        if (other.ok_) {
            storage_.raw_construct(std::move(other.storage_.template get<T>()));
            other.storage_.destroy_ok();
        } else {
            storage_.raw_construct(std::move(other.storage_.template get<E>()));
            other.storage_.destroy_err();
        }
    }

    template <typename T, typename E>
    Result<T, E>::Result(const Result<T, E>& other)
            : ok_(other.ok_)
            , storage_()
    {
        if (other.ok_) {
            storage_.raw_construct(other.storage_.template get<T>());
        } else {
            storage_.raw_construct(other.storage_.template get<E>());
        }
    }

    template <typename T, typename E>
    Result<T, E>& Result<T, E>::operator=(Result<T, E>&& other) noexcept
    {
        if (this != &other) {
            if (ok_) {
                storage_.destroy_ok();
                ok_ = other.ok_;
                if (other.ok_) {
                    storage_.raw_construct(std::move(other.storage_.template get<T>()));
                    other.storage_.destroy_ok();
                } else {
                    storage_.raw_construct(std::move(other.storage_.template get<E>()));
                    other.storage_.destroy_err();
                }
            } else {
                storage_.destroy_err();
                ok_ = other.ok_;
                if (other.ok_) {
                    storage_.raw_construct(std::move(other.storage_.template get<T>()));
                    other.storage_.destroy_ok();
                } else {
                    storage_.raw_construct(std::move(other.storage_.template get<E>()));
                    other.storage_.destroy_err();
                }
            }
        }
        return *this;
    }

    template <typename T, typename E>
    Result<T, E>& Result<T, E>::operator=(const Result<T, E>& other)
    {
        if (this != &other) {
            if (ok_) {
                storage_.destroy_ok();
                ok_ = other.ok_;
                if (other.ok_) {
                    storage_.raw_construct(other.storage_.template get<T>());
                } else {
                    storage_.raw_construct(other.storage_.template get<E>());
                }
            } else {
                storage_.destroy_err();
                ok_ = other.ok_;
                if (other.ok_) {
                    storage_.raw_construct(other.storage_.template get<T>());
                } else {
                    storage_.raw_construct(other.storage_.template get<E>());
                }
            }
        }
        return *this;
    }

    template <typename T, typename E>
    Result<T, E>::Result(types::Ok<T>&& ok) noexcept
            : ok_(true)
            , storage_()
    {
        storage_.construct(std::move(ok));
    }

    template <typename T, typename E>
    Result<T, E>::Result(types::Err<E>&& err) noexcept
            : ok_(false)
            , storage_()
    {
        storage_.construct(std::move(err));
    }

    template <typename T, typename E>
    bool Result<T, E>::is_ok() const
    {
        return ok_;
    }

    template <typename T, typename E>
    bool Result<T, E>::is_err() const
    {
        return !ok_;
    }

    template <typename T, typename E>
    template <typename U>
    Result<U, E> Result<T, E>::map(const std::function<U(const T&)>& f) const
    {
        if (ok_) {
            auto res = f(storage_.template get<T>());
            return types::Ok<U>(std::move(res));
        } else {
            return types::Err<E>(storage_.template get<E>());
        }
    }

    template <typename T, typename E>
    template <typename U>
    Result<U, E> Result<T, E>::map_or(U&& value, const std::function<U(const T&)>& f) const
    {
        if (ok_) {
            auto res = f(storage_.template get<T>());
            return types::Ok<U>(std::move(res));
        } else {
            return types::Ok<U>(value);
        }
    }

    template <typename T, typename E>
    template <typename U>
    Result<U, E> Result<T, E>::map_or_else(const std::function<U(const E&)>& provider, const std::function<U(const T&)>& f) const
    {
        if (ok_) {
            auto res = f(storage_.template get<T>());
            return types::Ok<U>(std::move(res));
        } else {
            auto res = provider(storage_.template get<E>());
            return types::Ok<U>(std::move(res));
        }
    }

    template <typename T, typename E>
    template <typename F>
    Result<T, F> Result<T, E>::map_err(const std::function<F(const E&)>& f) const
    {
        if (ok_) {
            auto res = storage_.template get<T>();
            return types::Ok<T>(std::move(res));
        } else {
            auto res = f(storage_.template get<E>());
            return types::Err<F>(std::move(res));
        }
    }

    template <typename T, typename E>
    template <typename U>
    Result<U, E> Result<T, E>::and_(const Result<U, E>& rhs) const
    {
        if (ok_) {
            return rhs;
        } else {
            auto res = storage_.template get<E>();
            return types::Err<E>(std::move(res));
        }
    }

    template <typename T, typename E>
    template <typename U>
    Result<U, E> Result<T, E>::and_then(const std::function<Result<U, E>(const T&)>& f) const
    {
        if (ok_) {
            return f(storage_.template get<T>());
        } else {
            return types::Err<E>(storage_.template get<E>());
        }
    }

    template <typename T, typename E>
    Result<T, E> Result<T, E>::or_(const Result<T, E>& rhs) const
    {
        if (ok_) {
            return *this;
        } else {
            return rhs;
        }
    }

    template <typename T, typename E>
    Result<T, E> Result<T, E>::or_else(const std::function<Result<T, E>(const E&)>& f) const
    {
        if (ok_) {
            return *this;
        } else {
            return f(storage_.template get<E>());
        }
    }

    template <typename T, typename E>
    const T& Result<T, E>::unwrap() const
    {
        return storage_.template get<T>();
    }

    template <typename T, typename E>
    const E& Result<T, E>::unwrap_err() const
    {
        return storage_.template get<E>();
    }

    template <typename T, typename E>
    const T& Result<T, E>::unwrap_or(const T& value) const
    {
        if (ok_) {
            return storage_.template get<T>();
        } else {
            return value;
        }
    }

    template <typename T, typename E>
    T Result<T, E>::unwrap_or_else(const std::function<T(const E&)>& provider) const
    {
        if (ok_) {
            return storage_.template get<T>();
        } else {
            return provider(storage_.template get<E>());
        }
    }

    template <typename T, typename E>
    const Result<T, E>& Result<T, E>::on_success(const std::function<void(const T&)>& f) const
    {
        if (ok_) {
            f(storage_.template get<T>());
        }
        return *this;
    }

    template <typename T, typename E>
    const Result<T, E>& Result<T, E>::on_error(const std::function<void(const E&)>& f) const
    {
        if (!ok_) {
            f(storage_.template get<E>());
        }
        return *this;
    }
}
