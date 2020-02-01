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

#include "intercept.h"

#include "Executor.h"
#include "Resolver.h"
#include "Session.h"

namespace {

    constexpr char LS_PATH[] = "/usr/bin/ls";
    constexpr char LS_FILE[] = "ls";
    constexpr char* LS_ARGV[] = {
        const_cast<char*>("/usr/bin/ls"),
        const_cast<char*>("-l"),
        nullptr
    };
    constexpr char* LS_ENVP[] = {
        const_cast<char*>("PATH=/usr/bin:/usr/sbin"),
        nullptr
    };
    constexpr char SEARCH_PATH[] = "/usr/bin:/usr/sbin";

    constexpr int FAILURE = -1;
    constexpr int SUCCESS = 0;

    class ExecutorTest : public ::testing::Test {
    public:
        static const ear::Session BROKEN_SESSION;
        static const ear::Session SILENT_SESSION;
        static const ear::Session VERBOSE_SESSION;

        static void* null_resolver(char const* const)
        {
            return nullptr;
        }

        static void* not_called(char const* const)
        {
            EXPECT_TRUE(false);
            return nullptr;
        }

        static ear::Resolver::execve_t mock_silent_session_execve() noexcept
        {
            return [](const char* path, char* const argv[], char* const envp[]) -> int {
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), path);
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(SILENT_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(SILENT_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::PATH, argv[5]);
                EXPECT_STREQ(LS_PATH, argv[6]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[7]);
                EXPECT_STREQ(LS_ARGV[0], argv[8]);
                EXPECT_STREQ(LS_ARGV[1], argv[9]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static ear::Resolver::execve_t mock_verbose_session_execve() noexcept
        {
            return [](const char* path, char* const argv[], char* const envp[]) -> int {
                EXPECT_STREQ(VERBOSE_SESSION.get_reporter(), path);
                EXPECT_STREQ(VERBOSE_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(VERBOSE_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(VERBOSE_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::VERBOSE, argv[5]);
                EXPECT_STREQ(pear::flag::PATH, argv[6]);
                EXPECT_STREQ(LS_PATH, argv[7]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[8]);
                EXPECT_STREQ(LS_ARGV[0], argv[9]);
                EXPECT_STREQ(LS_ARGV[1], argv[10]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static ear::Resolver::execve_t mock_silent_session_execvpe() noexcept
        {
            return [](const char* path, char* const argv[], char* const envp[]) -> int {
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), path);
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(SILENT_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(SILENT_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::FILE, argv[5]);
                EXPECT_STREQ(LS_FILE, argv[6]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[7]);
                EXPECT_STREQ(LS_ARGV[0], argv[8]);
                EXPECT_STREQ(LS_ARGV[1], argv[9]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static ear::Resolver::execve_t mock_silent_session_execvp2() noexcept
        {
            return [](const char* path, char* const argv[], char* const envp[]) -> int {
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), path);
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(SILENT_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(SILENT_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::FILE, argv[5]);
                EXPECT_STREQ(LS_FILE, argv[6]);
                EXPECT_STREQ(pear::flag::SEARCH_PATH, argv[7]);
                EXPECT_STREQ(SEARCH_PATH, argv[8]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[9]);
                EXPECT_STREQ(LS_ARGV[0], argv[10]);
                EXPECT_STREQ(LS_ARGV[1], argv[11]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static ear::Resolver::posix_spawn_t mock_silent_session_spawn() noexcept
        {
            return [](pid_t* pid,
                       const char* path,
                       const posix_spawn_file_actions_t* file_actions,
                       const posix_spawnattr_t* attrp,
                       char* const argv[],
                       char* const envp[]) -> int {
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), path);
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(SILENT_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(SILENT_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::PATH, argv[5]);
                EXPECT_STREQ(LS_PATH, argv[6]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[7]);
                EXPECT_STREQ(LS_ARGV[0], argv[8]);
                EXPECT_STREQ(LS_ARGV[1], argv[9]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static ear::Resolver::posix_spawn_t mock_silent_session_spawnp() noexcept
        {
            return [](pid_t* pid,
                       const char* path,
                       const posix_spawn_file_actions_t* file_actions,
                       const posix_spawnattr_t* attrp,
                       char* const argv[],
                       char* const envp[]) -> int {
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), path);
                EXPECT_STREQ(SILENT_SESSION.get_reporter(), argv[0]);
                EXPECT_STREQ(pear::flag::DESTINATION, argv[1]);
                EXPECT_STREQ(SILENT_SESSION.get_destination(), argv[2]);
                EXPECT_STREQ(pear::flag::LIBRARY, argv[3]);
                EXPECT_STREQ(SILENT_SESSION.get_library(), argv[4]);
                EXPECT_STREQ(pear::flag::FILE, argv[5]);
                EXPECT_STREQ(LS_FILE, argv[6]);
                EXPECT_STREQ(pear::flag::COMMAND, argv[7]);
                EXPECT_STREQ(LS_ARGV[0], argv[8]);
                EXPECT_STREQ(LS_ARGV[1], argv[9]);
                EXPECT_EQ(LS_ENVP, envp);
                return SUCCESS;
            };
        }

        static const ear::Resolver BROKEN_RESOLVER;
        static const ear::Resolver IGNORED_RESOLVER;
        static const ear::Resolver MOCK_SILENT_EXECVE_RESOLVER;
        static const ear::Resolver MOCK_VERBOSE_EXECVE_RESOLVER;
        static const ear::Resolver MOCK_SILENT_EXECVPE_RESOLVER;
        static const ear::Resolver MOCK_SILENT_EXECVP2_RESOLVER;
        static const ear::Resolver MOCK_SILENT_SPAWN_RESOLVER;
        static const ear::Resolver MOCK_SILENT_SPAWNP_RESOLVER;
    };
    const ear::Session ExecutorTest::BROKEN_SESSION = {};
    const ear::Session ExecutorTest::SILENT_SESSION = {
        "/usr/libexec/libexec.so",
        "/usr/bin/intercept",
        "/tmp/intercept.random",
        false
    };
    const ear::Session ExecutorTest::VERBOSE_SESSION = {
        "/usr/libexec/libexec.so",
        "/usr/bin/intercept",
        "/tmp/intercept.random",
        true
    };
    const ear::Resolver ExecutorTest::BROKEN_RESOLVER = ear::Resolver(&ExecutorTest::null_resolver);
    const ear::Resolver ExecutorTest::IGNORED_RESOLVER = ear::Resolver(&ExecutorTest::not_called);
    const ear::Resolver ExecutorTest::MOCK_SILENT_EXECVE_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_silent_session_execve));
    const ear::Resolver ExecutorTest::MOCK_VERBOSE_EXECVE_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_verbose_session_execve));
    const ear::Resolver ExecutorTest::MOCK_SILENT_EXECVPE_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_silent_session_execvpe));
    const ear::Resolver ExecutorTest::MOCK_SILENT_EXECVP2_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_silent_session_execvp2));
    const ear::Resolver ExecutorTest::MOCK_SILENT_SPAWN_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_silent_session_spawn));
    const ear::Resolver ExecutorTest::MOCK_SILENT_SPAWNP_RESOLVER = ear::Resolver(reinterpret_cast<ear::Resolver::resolver_t>(ExecutorTest::mock_silent_session_spawnp));

    TEST_F(ExecutorTest, execve_fails_without_env)
    {
        auto result = ear::Executor(BROKEN_SESSION, IGNORED_RESOLVER).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execve_fails_without_resolver)
    {
        auto result = ear::Executor(SILENT_SESSION, BROKEN_RESOLVER).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execve_silent_library)
    {
        auto result = ear::Executor(SILENT_SESSION, MOCK_SILENT_EXECVE_RESOLVER).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execve_verbose_library)
    {
        auto result = ear::Executor(VERBOSE_SESSION, MOCK_VERBOSE_EXECVE_RESOLVER).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_env)
    {
        auto result = ear::Executor(BROKEN_SESSION, IGNORED_RESOLVER).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_resolver)
    {
        auto result = ear::Executor(SILENT_SESSION, BROKEN_RESOLVER).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvpe_passes)
    {
        auto result = ear::Executor(SILENT_SESSION, MOCK_SILENT_EXECVPE_RESOLVER).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_env)
    {
        auto result = ear::Executor(BROKEN_SESSION, IGNORED_RESOLVER).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_resolver)
    {
        auto result = ear::Executor(SILENT_SESSION, BROKEN_RESOLVER).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvp2_passes)
    {
        auto result = ear::Executor(SILENT_SESSION, MOCK_SILENT_EXECVP2_RESOLVER).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_env)
    {
        pid_t pid;
        auto result = ear::Executor(BROKEN_SESSION, IGNORED_RESOLVER).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_resolver)
    {
        pid_t pid;
        auto result = ear::Executor(SILENT_SESSION, BROKEN_RESOLVER).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawn_passes)
    {
        pid_t pid;
        auto result = ear::Executor(SILENT_SESSION, MOCK_SILENT_SPAWN_RESOLVER).posix_spawn(&pid, LS_PATH, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_env)
    {
        pid_t pid;
        auto result = ear::Executor(BROKEN_SESSION, IGNORED_RESOLVER).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_resolver)
    {
        pid_t pid;
        auto result = ear::Executor(SILENT_SESSION, BROKEN_RESOLVER).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawnp_passes)
    {
        pid_t pid;
        auto result = ear::Executor(SILENT_SESSION, MOCK_SILENT_SPAWNP_RESOLVER).posix_spawnp(&pid, LS_FILE, nullptr, nullptr, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

}