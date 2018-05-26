#include "gtest/gtest.h"

#include "libear_a/Interface.h"
#include "libear_a/Environment.h"
#include "libear_a/Executor.h"

namespace {

    constexpr char ls_path[] = "/usr/bin/ls";
    constexpr char* ls_argv[] = {
            const_cast<char *>("/usr/bin/ls"),
            const_cast<char *>("-l"),
            nullptr
    };
    constexpr char* ls_envp[] = {
            const_cast<char *>("PATH=/usr/bin:/usr/sbin"),
            nullptr
    };

    constexpr char destination_str[] = "/tmp/pear.random";
    constexpr char library_str[] = "/usr/libexec/libear.so";
    constexpr char reporter_str[] = "/usr/bin/pear";

    constexpr int failure = -1;
    constexpr int success = 0;

    class ExecutorTest
            : public ::ear::LibrarySession
            , public ::testing::Test {
    public:
        ExecutorTest()
        {
            session.reporter = reporter_str;
            session.destination = destination_str;
            session.verbose = true;
            library = library_str;
        }
    };

    TEST_F(ExecutorTest, execve_return_error_without_env) {
        struct Validator {
            using execve_t = int (*)(const char *path, char *const argv[], char *const envp[]);
            static execve_t execve() {
                return nullptr;
            }
        };
        using Sut = ::ear::Executor < Validator>;

        auto result = Sut(nullptr).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execve_return_result_without_env) {
        struct Validator {
            static auto execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_EQ(ls_path, path);
                    EXPECT_EQ(ls_argv, argv);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        auto result = Sut(nullptr).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execve_return_result_with_env) {
        struct Validator {
            static auto execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(reporter_str, path);
                    EXPECT_STREQ(reporter_str, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(destination_str, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(library_str, argv[4]);
                    EXPECT_STREQ(ear::command_flag, argv[5]);
                    EXPECT_STREQ(ls_argv[0], argv[6]);
                    EXPECT_STREQ(ls_argv[1], argv[7]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        auto result = Sut(this).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

}