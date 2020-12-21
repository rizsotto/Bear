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

#include "report/libexec/Executor.h"
#include "ResolverMock.h"
#include "LinkerMock.h"
#include "report/libexec/Session.h"
#include "report/libexec/Array.h"
#include "report/supervisor/Flags.h"

#include <cerrno>

using ::testing::_;
using ::testing::Args;
using ::testing::ElementsAre;
using ::testing::ElementsAreArray;
using ::testing::Matcher;
using ::testing::NotNull;
using ::testing::Return;

namespace {

    const char* LS_PATH = "/usr/bin/ls";
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

    MATCHER_P(CStyleArrayEqual, expecteds, "")
    {
        size_t idx = 0;
        for (const auto &expected: expecteds) {
            if (std::string_view(arg[idx]) != std::string_view(expected)) {
                *result_listener << "expected: " << expected << ", but got: " << arg[idx];
                return false;
            }
            ++idx;
        }
        return true;
    }

    TEST(Executor, fails_without_session)
    {
        const rust::Result<int, int> expected = rust::Err(EIO);

        el::Session session;

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(_)).Times(0);
        EXPECT_CALL(resolver, from_path(_, _)).Times(0);
        EXPECT_CALL(resolver, from_search_path(_, _)).Times(0);

        EXPECT_EQ(expected, el::Executor(linker, session, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP));
        EXPECT_EQ(expected, el::Executor(linker, session, resolver).execvpe(LS_FILE, LS_ARGV, LS_ENVP));
        EXPECT_EQ(expected, el::Executor(linker, session, resolver).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP));

        pid_t pid;
        EXPECT_EQ(expected, el::Executor(linker, session, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP));
        EXPECT_EQ(expected, el::Executor(linker, session, resolver).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP));
    }

    TEST(Executor, execve_silent_library)
    {
        const rust::Result<int, int> expected = rust::Ok(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker,execve(SILENT_SESSION.reporter,
                                  CStyleArrayEqual(std::vector<const char *> {
                                      SILENT_SESSION.reporter,
                                      er::DESTINATION,
                                      SILENT_SESSION.destination,
                                      er::EXECUTE,
                                      LS_PATH,
                                      er::COMMAND,
                                      LS_ARGV[0],
                                      LS_ARGV[1]
                                  }),
                                  LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, SILENT_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, execve_verbose_library)
    {
        const rust::Result<int, int> expected = rust::Ok(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter,
                                   CStyleArrayEqual(std::vector<const char *> {
                                           VERBOSE_SESSION.reporter,
                                           er::DESTINATION,
                                           VERBOSE_SESSION.destination,
                                           er::VERBOSE,
                                           er::EXECUTE,
                                           LS_PATH,
                                           er::COMMAND,
                                           LS_ARGV[0],
                                           LS_ARGV[1]
                                   }),
                                   LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, execvpe_fails_on_resolve)
    {
        const rust::Result<int, int> expected = rust::Err(ENOENT);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Err(ENOENT))));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        auto result = el::Executor(linker, SILENT_SESSION, resolver).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, execvpe_passes)
    {
        const rust::Result<int, int> expected = rust::Ok(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(LS_ENVP)))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter,
                                   CStyleArrayEqual(std::vector<const char *> {
                                       VERBOSE_SESSION.reporter,
                                       er::DESTINATION,
                                       VERBOSE_SESSION.destination,
                                       er::VERBOSE,
                                       er::EXECUTE,
                                       LS_PATH,
                                       er::COMMAND,
                                       LS_ARGV[0],
                                       LS_ARGV[1]
                                   }),
                                   LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, execvp2_passes)
    {
        const rust::Result<int, int> expected = rust::Ok(0);

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_search_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(SEARCH_PATH)))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(VERBOSE_SESSION.reporter,
                                   CStyleArrayEqual(std::vector<const char *> {
                                           VERBOSE_SESSION.reporter,
                                           er::DESTINATION,
                                           VERBOSE_SESSION.destination,
                                           er::VERBOSE,
                                           er::EXECUTE,
                                           LS_PATH,
                                           er::COMMAND,
                                           LS_ARGV[0],
                                           LS_ARGV[1]
                                   }),
                                   LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, spawn_passes)
    {
        const rust::Result<int, int> expected = rust::Ok(0);
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker, posix_spawn(&pid, VERBOSE_SESSION.reporter, nullptr, nullptr,
                                        CStyleArrayEqual(std::vector<const char *> {
                                                VERBOSE_SESSION.reporter,
                                                er::DESTINATION,
                                                VERBOSE_SESSION.destination,
                                                er::VERBOSE,
                                                er::EXECUTE,
                                                LS_PATH,
                                                er::COMMAND,
                                                LS_ARGV[0],
                                                LS_ARGV[1]
                                        }),
                                        LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, spawn_fails_on_access)
    {
        const rust::Result<int, int> expected = rust::Err(ENOENT);
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_current_directory(testing::Eq(std::string_view(LS_PATH))))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Err(ENOENT))));

        LinkerMock linker;
        EXPECT_CALL(linker, execve(_, _, _)).Times(0);
        EXPECT_CALL(linker, posix_spawn(_, _, _, _, _, _)).Times(0);

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }

    TEST(Executor, spawnp_passes)
    {
        const rust::Result<int, int> expected = rust::Ok(0);
        pid_t pid;

        ResolverMock resolver;
        EXPECT_CALL(resolver, from_path(testing::Eq(std::string_view(LS_FILE)), testing::Eq(LS_ENVP)))
                .Times(1)
                .WillOnce(Return(rust::Result<const char*, int>(rust::Ok(LS_PATH))));

        LinkerMock linker;
        EXPECT_CALL(linker, posix_spawn(&pid, VERBOSE_SESSION.reporter, nullptr, nullptr,
                                        CStyleArrayEqual(std::vector<const char *> {
                                                VERBOSE_SESSION.reporter,
                                                er::DESTINATION,
                                                VERBOSE_SESSION.destination,
                                                er::VERBOSE,
                                                er::EXECUTE,
                                                LS_PATH,
                                                er::COMMAND,
                                                LS_ARGV[0],
                                                LS_ARGV[1]
                                        }),
                                        LS_ENVP))
                .Times(1)
                .WillOnce(Return(expected));

        auto result = el::Executor(linker, VERBOSE_SESSION, resolver).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(expected, result);
    }
}
