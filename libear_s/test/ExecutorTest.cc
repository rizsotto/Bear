#include "gtest/gtest.h"

#include "../source/Environment.h"
#include "../source/Executor.h"

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

    constexpr char target_str[] = "/tmp/pear.random";
    constexpr char library_str[] = "/usr/libexec/libear.so";
    constexpr char wrapper_str[] = "/usr/bin/pear";

    constexpr int failure = -1;
    constexpr int success = 0;

    class ExecutorTest
            : public ::ear::Environment
            , public ::testing::Test {
    public:
        ExecutorTest()
                : ::ear::Environment(target_str, library_str, wrapper_str)
        {}
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
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execve_return_result_with_env) {
        struct Validator {
            static auto execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(wrapper_str, path);
                    EXPECT_STREQ(wrapper_str, argv[0]);
                    EXPECT_STREQ("-t", argv[1]);
                    EXPECT_STREQ(target_str, argv[2]);
                    EXPECT_STREQ("-l", argv[3]);
                    EXPECT_STREQ(library_str, argv[4]);
                    EXPECT_STREQ("-m", argv[5]);
                    EXPECT_STREQ("execve", argv[6]);
                    EXPECT_STREQ(ls_argv[0], argv[7]);
                    EXPECT_STREQ(ls_argv[1], argv[8]);
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