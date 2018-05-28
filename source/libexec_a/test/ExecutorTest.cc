#include "gtest/gtest.h"

#include "../Interface.h"
#include "../Environment.h"
#include "../Executor.h"

namespace {

    constexpr char ls_path[] = "/usr/bin/ls";
    constexpr char ls_file[] = "ls";
    constexpr char* ls_argv[] = {
            const_cast<char *>("/usr/bin/ls"),
            const_cast<char *>("-l"),
            nullptr
    };
    constexpr char* ls_envp[] = {
            const_cast<char *>("PATH=/usr/bin:/usr/sbin"),
            nullptr
    };
    constexpr char search_path[] = "/usr/bin:/usr/sbin";

    constexpr int failure = -1;
    constexpr int success = 0;

    struct BrokenResolver {
        using execve_t = int (*)(const char *path, char *const argv[], char *const envp[]);
        static execve_t resolve_execve() {
            return nullptr;
        }
        using posix_spawn_t = int (*)(pid_t *pid,
                                      const char *path,
                                      const posix_spawn_file_actions_t *file_actions,
                                      const posix_spawnattr_t *attrp,
                                      char *const argv[],
                                      char *const envp[]);
        static posix_spawn_t resolve_spawn() noexcept {
            return nullptr;
        }
    };

    class ExecutorTest : public ::testing::Test {
    public:
        static constexpr ::ear::Session silent_session = {
                "/usr/bin/intercept",
                "/tmp/intercept.random",
                false
        };
        static constexpr ::ear::Session verbose_session = {
                "/usr/bin/intercept",
                "/tmp/intercept.random",
                true
        };
        static constexpr ::ear::LibrarySession silent_libray_session = {
                silent_session,
                "/usr/libexec/libexec.so"
        };
        static constexpr ::ear::LibrarySession verbose_libray_session = {
                verbose_session,
                "/usr/libexec/libexec.so"
        };
    };

    TEST_F(ExecutorTest, execve_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execve_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&silent_libray_session).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execve_silent_library) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(silent_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::command_flag, argv[5]);
                    EXPECT_STREQ(ls_argv[0], argv[6]);
                    EXPECT_STREQ(ls_argv[1], argv[7]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&silent_libray_session).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execve_verbose_library) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(verbose_session.reporter, path);
                    EXPECT_STREQ(verbose_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(verbose_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(verbose_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::verbose_flag, argv[5]);
                    EXPECT_STREQ(ear::command_flag, argv[6]);
                    EXPECT_STREQ(ls_argv[0], argv[7]);
                    EXPECT_STREQ(ls_argv[1], argv[8]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&verbose_libray_session).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execve_silent_wrapper) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::command_flag, argv[3]);
                    EXPECT_STREQ(ls_argv[0], argv[4]);
                    EXPECT_STREQ(ls_argv[1], argv[5]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&silent_session).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execve_verbose_wrapper) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(verbose_session.reporter, path);
                    EXPECT_STREQ(verbose_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(verbose_session.destination, argv[2]);
                    EXPECT_STREQ(ear::verbose_flag, argv[3]);
                    EXPECT_STREQ(ear::command_flag, argv[4]);
                    EXPECT_STREQ(ls_argv[0], argv[5]);
                    EXPECT_STREQ(ls_argv[1], argv[6]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&verbose_session).execve(ls_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execvpe(ls_file, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&silent_libray_session).execvpe(ls_file, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execvpe_passes) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(silent_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::file_flag, argv[5]);
                    EXPECT_STREQ(ls_file, argv[6]);
                    EXPECT_STREQ(ear::command_flag, argv[7]);
                    EXPECT_STREQ(ls_argv[0], argv[8]);
                    EXPECT_STREQ(ls_argv[1], argv[9]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&silent_libray_session).execvpe(ls_file, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execvP(ls_file, search_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&silent_libray_session).execvP(ls_file, search_path, ls_argv, ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, execvp2_passes) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(silent_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::file_flag, argv[5]);
                    EXPECT_STREQ(ls_file, argv[6]);
                    EXPECT_STREQ(ear::search_flag, argv[7]);
                    EXPECT_STREQ(search_path, argv[8]);
                    EXPECT_STREQ(ear::command_flag, argv[9]);
                    EXPECT_STREQ(ls_argv[0], argv[10]);
                    EXPECT_STREQ(ls_argv[1], argv[11]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&silent_libray_session).execvP(ls_file, search_path, ls_argv, ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        pid_t pid;
        auto result = Sut(session_ptr).posix_spawn(
                &pid,
                ls_path,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;


        pid_t pid;
        auto result = Sut(&silent_libray_session).posix_spawn(
                &pid,
                ls_path,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, spawn_passes) {
        struct Validator {
            static auto resolve_spawn() {
                return [](pid_t *pid,
                          const char *path,
                          const posix_spawn_file_actions_t *file_actions,
                          const posix_spawnattr_t *attrp,
                          char *const argv[],
                          char *const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(silent_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::command_flag, argv[5]);
                    EXPECT_STREQ(ls_argv[0], argv[6]);
                    EXPECT_STREQ(ls_argv[1], argv[7]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        pid_t pid;
        auto result = Sut(&silent_libray_session).posix_spawn(
                &pid,
                ls_path,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(success, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        pid_t pid;
        auto result = Sut(session_ptr).posix_spawnp(
                &pid,
                ls_file,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;


        pid_t pid;
        auto result = Sut(&silent_libray_session).posix_spawnp(
                &pid,
                ls_file,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(failure, result);
    }

    TEST_F(ExecutorTest, spawnp_passes) {
        struct Validator {
            static auto resolve_spawn() {
                return [](pid_t *pid,
                          const char *path,
                          const posix_spawn_file_actions_t *file_actions,
                          const posix_spawnattr_t *attrp,
                          char *const argv[],
                          char *const envp[]) -> int {
                    EXPECT_STREQ(silent_session.reporter, path);
                    EXPECT_STREQ(silent_session.reporter, argv[0]);
                    EXPECT_STREQ(ear::destination_flag, argv[1]);
                    EXPECT_STREQ(silent_session.destination, argv[2]);
                    EXPECT_STREQ(ear::library_flag, argv[3]);
                    EXPECT_STREQ(silent_libray_session.library, argv[4]);
                    EXPECT_STREQ(ear::file_flag, argv[5]);
                    EXPECT_STREQ(ls_file, argv[6]);
                    EXPECT_STREQ(ear::command_flag, argv[7]);
                    EXPECT_STREQ(ls_argv[0], argv[8]);
                    EXPECT_STREQ(ls_argv[1], argv[9]);
                    EXPECT_EQ(ls_envp, envp);
                    return success;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        pid_t pid;
        auto result = Sut(&silent_libray_session).posix_spawnp(
                &pid,
                ls_file,
                nullptr,
                nullptr,
                ls_argv,
                ls_envp);
        EXPECT_EQ(success, result);
    }

}