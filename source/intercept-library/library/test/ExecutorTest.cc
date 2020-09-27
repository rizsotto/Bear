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

#include "gtest/gtest.h"

#include "Executor.h"
#include "ResolverMock.h"
#include "LinkerMock.h"
#include "Session.h"
#include "Array.h"

#include <cerrno>

using ::testing::_;
using ::testing::Args;
using ::testing::ElementsAre;
using ::testing::ElementsAreArray;
using ::testing::Matcher;
using ::testing::NotNull;
using ::testing::Return;

namespace el {

    bool operator==(const Executor::Result &lhs, const Executor::Result &rhs)
    {
        return
            (lhs.return_value == rhs.return_value) &&
            (lhs.return_value == 0 || lhs.error_code == rhs.error_code);
    }

    std::ostream& operator<<(std::ostream &os, const Executor::Result &value)
    {
        os << "return value: " << value.return_value << ", error code: " << value.error_code;
        return os;
    }
}

namespace {

    char* LS_PATH = const_cast<char*>("/usr/bin/ls");
    size_t LS_PATH_SIZE = el::array::length(LS_PATH);
    char* LS_FILE = const_cast<char*>("ls");
    char* LS_ARGV[] = {
        const_cast<char*>("ls"),
        const_cast<char*>("-l"),
        nullptr
    };
    char* LS_ENVP[] = {
        const_cast<char*>("PATH=/usr/bin:/usr/sbin"),
        nullptr
    };
    char SEARCH_PATH[] = "/usr/bin:/usr/sbin";

    el::Session SILENT_SESSION = {
        "/usr/bin/intercept",
        "/tmp/intercept.random",
        false
    };

    el::Session VERBOSE_SESSION = {
        "/usr/bin/intercept",
        "/tmp/intercept.random",
        true
    };

    constexpr el::Executor::Result SUCCESS = { 0, 0 };

    constexpr el::Executor::Result failure(int const error_code) noexcept
    {
        return el::Executor::Result { -1, error_code };
    }

    TEST(Executor, fails_without_session)
    {
        el::Session session = el::session::init();

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(_)).Times(0);
        EXPECT_CALL(resolver, from_path(_, _)).Times(0);
        EXPECT_CALL(resolver, from_search_path(_, _)).Times(0);

        EXPECT_EQ(failure(EIO), el::Executor(linker, session, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP));
        EXPECT_EQ(failure(EIO), el::Executor(linker, session, resolver).execvpe(LS_FILE, LS_ARGV, LS_ENVP));
        EXPECT_EQ(failure(EIO), el::Executor(linker, session, resolver).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP));

        pid_t pid;
        EXPECT_EQ(failure(EIO), el::Executor(linker, session, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP));
        EXPECT_EQ(failure(EIO), el::Executor(linker, session, resolver).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP));
    }

    TEST(Executor, execve_silent_library)
    {
        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        session.reporter,
        //        pear::flag::DESTINATION,
        //        session.destination,
        //        pear::flag::PATH,
        //        LS_PATH,
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, execve(SILENT_SESSION.reporter, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, SILENT_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST(Executor, execve_verbose_library)
    {
        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        session.reporter,
        //        pear::flag::VERBOSE,
        //        pear::flag::DESTINATION,
        //        session.destination,
        //        pear::flag::LIBRARY,
        //        LS_PATH,
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST(Executor, execvpe_fails_on_resolve)
    {
        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{nullptr, ENOENT}));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        auto result = el::Executor(linker, SILENT_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(failure(ENOENT), result);
    }

    TEST(Executor, execvpe_passes)
    {
        ResolverMock resolver;
        EXPECT_CALL(resolver, from_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(LS_ENVP)))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        SILENT_SESSION.reporter,
        //        pear::flag::VERBOSE,
        //        pear::flag::DESTINATION,
        //        SILENT_SESSION.destination,
        //        pear::flag::FILE,
        //        LS_FILE,
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST(Executor, execvp2_passes)
    {
        ResolverMock resolver;
        EXPECT_CALL(resolver, from_search_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(SEARCH_PATH)))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        SILENT_SESSION.reporter,
        //        pear::flag::VERBOSE,
        //        pear::flag::DESTINATION,
        //        SILENT_SESSION.destination,
        //        pear::flag::FILE,
        //        LS_FILE,
        //        pear::flag::SEARCH_PATH
        //        SEARCH_PATH
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST(Executor, spawn_passes)
    {
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        VERBOSE_SESSION.reporter,
        //        pear::flag::VERBOSE,
        //        pear::flag::DESTINATION,
        //        VERBOSE_SESSION.destination,
        //        pear::flag::PATH,
        //        LS_PATH,
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, posix_spawn(&pid, VERBOSE_SESSION.reporter, nullptr, nullptr, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST(Executor, spawn_fails_on_access)
    {
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{nullptr, ENOENT}));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(failure(ENOENT), result);
    }

    TEST(Executor, spawnp_passes)
    {
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(LS_ENVP)))
                .Times(1)
                .WillOnce(Return(el::Resolver::Result{LS_PATH, 0}));

        LinkerMock linker;
        // TODO: verify the arguments
        //    const char* argv[] = {
        //        VERBOSE_SESSION.reporter,
        //        pear::flag::VERBOSE,
        //        pear::flag::DESTINATION,
        //        VERBOSE_SESSION.destination,
        //        pear::flag::FILE,
        //        LS_FILE,
        //        pear::flag::COMMAND,
        //        LS_ARGV[0],
        //        LS_ARGV[1],
        //        nullptr
        //    };
        EXPECT_CALL(linker, posix_spawn(&pid, VERBOSE_SESSION.reporter, nullptr, nullptr, NotNull(), LS_ENVP))
                .Times(1)
                .WillOnce(Return(0));
        EXPECT_CALL(linker, error_code())
                .WillRepeatedly(Return(0));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }
}
