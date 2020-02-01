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

namespace ear {

    class Storage;

    /**
     * Represents an intercept session parameter set.
     */
    class Session {
    public:
        Session() noexcept = default;

        Session(char const *library, char const *reporter, char const *destination, bool) noexcept;

        ~Session() noexcept = default;

        Session(Session const &) = delete;

        Session(Session &&) noexcept = default;

        Session &operator=(Session const &) = delete;

        Session &operator=(Session &&) noexcept = default;

        /**
         * Creates a Session object from the given environment.
         *
         * @param environment to initialize from.
         * @return a Session object which might or might not be initialized.
         */
        static Session from(const char **environment) noexcept;

    public:
        const char *get_library() const;

        const char *get_reporter() const;

        const char *get_destination() const;

        bool is_verbose() const;

    public:
        /**
         * @return true if the session is initialized and can be used to
         * intercept execution calls.
         */
        bool is_not_valid() const noexcept;

        /**
         * It persist the parameters to a buffer.
         *
         * If the values were created from the environment array. Those
         * pointers can be freed before they get used. (The process calls
         * a `setenv` method.)
         *
         * @param storage uses a buffer to persist the values.
         */
        void persist(Storage &storage) noexcept;

        /**
         * Report a function call to stderr.
         *
         * It's for debugging purposes.
         *
         * @param message to print.
         */
        void write_message(const char *message) const noexcept;

    private:
        char const *library_;
        char const *reporter_;
        char const *destination_;
        bool verbose_;
    };

    inline
    Session::Session(char const *library, char const *reporter, char const *destination, bool verbose) noexcept
            : library_(library)
            , reporter_(reporter)
            , destination_(destination)
            , verbose_(verbose)
    { }

    inline
    const char *Session::get_library() const {
        return library_;
    }

    inline
    const char *Session::get_reporter() const {
        return reporter_;
    }

    inline
    const char *Session::get_destination() const {
        return destination_;
    }

    inline
    bool Session::is_verbose() const {
        return verbose_;
    }

    inline
    bool Session::is_not_valid() const noexcept {
        return (library_ == nullptr || reporter_ == nullptr || destination_ == nullptr);
    }
}
