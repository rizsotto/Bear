#include "gtest/gtest.h"

#include "../Interface.h"
#include "../Environment.h"
#include "../Executor.h"

namespace {

    constexpr char LS_PATH[] = "/usr/bin/ls";
    constexpr char LS_FILE[] = "ls";
    constexpr char* LS_ARGV[] = {
            const_cast<char *>("/usr/bin/ls"),
            const_cast<char *>("-l"),
            nullptr
    };
    constexpr char* LS_ENVP[] = {
            const_cast<char *>("PATH=/usr/bin:/usr/sbin"),
            nullptr
    };
    constexpr char SEARCH_PATH[] = "/usr/bin:/usr/sbin";

    constexpr int FAILURE = -1;
    constexpr int SUCCESS = 0;

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
        static const ::pear::Context SILENT_SESSION;
        static const ::pear::Context VERBOSE_SESSION;
        static const ::ear::LibrarySession SILENT_LIBRARY_SESSION;
        static const ::ear::LibrarySession VERBOSE_LIBRARY_SESSION;
        static const ::ear::WrapperSession SILENT_WRAPPER_SESSION;
        static const ::ear::WrapperSession VERBOSE_WRAPPER_SESSION;
    };
    const ::pear::Context ExecutorTest::SILENT_SESSION = {
            "/usr/bin/intercept",
            "/tmp/intercept.random",
            false
    };
    const ::pear::Context ExecutorTest::VERBOSE_SESSION = {
            "/usr/bin/intercept",
            "/tmp/intercept.random",
            true
    };
    const ::ear::LibrarySession ExecutorTest::SILENT_LIBRARY_SESSION = {
            SILENT_SESSION,
            "/usr/libexec/libexec.so"
    };
    const ::ear::LibrarySession ExecutorTest::VERBOSE_LIBRARY_SESSION = {
            VERBOSE_SESSION,
            "/usr/libexec/libexec.so"
    };
    const ::ear::WrapperSession ExecutorTest::SILENT_WRAPPER_SESSION = {
            SILENT_SESSION,
            "cc",
            "c++"
    };
    const ::ear::WrapperSession ExecutorTest::VERBOSE_WRAPPER_SESSION = {
            VERBOSE_SESSION,
            "cc",
            "c++"
    };

    TEST_F(ExecutorTest, execve_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execve_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execve_silent_library) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(SILENT_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::path, argv[5]);
                    EXPECT_STREQ(LS_PATH, argv[6]);
                    EXPECT_STREQ(::pear::flag::command, argv[7]);
                    EXPECT_STREQ(LS_ARGV[0], argv[8]);
                    EXPECT_STREQ(LS_ARGV[1], argv[9]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execve_verbose_library) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(VERBOSE_SESSION.reporter, path);
                    EXPECT_STREQ(VERBOSE_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(VERBOSE_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(VERBOSE_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::verbose, argv[5]);
                    EXPECT_STREQ(::pear::flag::path, argv[6]);
                    EXPECT_STREQ(LS_PATH, argv[7]);
                    EXPECT_STREQ(::pear::flag::command, argv[8]);
                    EXPECT_STREQ(LS_ARGV[0], argv[9]);
                    EXPECT_STREQ(LS_ARGV[1], argv[10]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&VERBOSE_LIBRARY_SESSION).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execve_silent_wrapper) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::path, argv[3]);
                    EXPECT_STREQ(LS_PATH, argv[4]);
                    EXPECT_STREQ(::pear::flag::command, argv[5]);
                    EXPECT_STREQ(LS_ARGV[0], argv[6]);
                    EXPECT_STREQ(LS_ARGV[1], argv[7]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&SILENT_WRAPPER_SESSION).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execve_verbose_wrapper) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(VERBOSE_SESSION.reporter, path);
                    EXPECT_STREQ(VERBOSE_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(VERBOSE_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::verbose, argv[3]);
                    EXPECT_STREQ(::pear::flag::path, argv[4]);
                    EXPECT_STREQ(LS_PATH, argv[5]);
                    EXPECT_STREQ(::pear::flag::command, argv[6]);
                    EXPECT_STREQ(LS_ARGV[0], argv[7]);
                    EXPECT_STREQ(LS_ARGV[1], argv[8]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&VERBOSE_WRAPPER_SESSION).execve(LS_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvpe_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvpe_passes) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(SILENT_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::file, argv[5]);
                    EXPECT_STREQ(LS_FILE, argv[6]);
                    EXPECT_STREQ(::pear::flag::command, argv[7]);
                    EXPECT_STREQ(LS_ARGV[0], argv[8]);
                    EXPECT_STREQ(LS_ARGV[1], argv[9]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execvpe(LS_FILE, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        auto result = Sut(session_ptr).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvp2_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, execvp2_passes) {
        struct Validator {
            static auto resolve_execve() {
                return [](const char* path, char* const argv[], char* const envp[]) -> int {
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(SILENT_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::file, argv[5]);
                    EXPECT_STREQ(LS_FILE, argv[6]);
                    EXPECT_STREQ(::pear::flag::search_path, argv[7]);
                    EXPECT_STREQ(SEARCH_PATH, argv[8]);
                    EXPECT_STREQ(::pear::flag::command, argv[9]);
                    EXPECT_STREQ(LS_ARGV[0], argv[10]);
                    EXPECT_STREQ(LS_ARGV[1], argv[11]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor<Validator>;

        auto result = Sut(&SILENT_LIBRARY_SESSION).execvP(LS_FILE, SEARCH_PATH, LS_ARGV, LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        pid_t pid;
        auto result = Sut(session_ptr).posix_spawn(
                &pid,
                LS_PATH,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawn_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;


        pid_t pid;
        auto result = Sut(&SILENT_LIBRARY_SESSION).posix_spawn(
                &pid,
                LS_PATH,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(FAILURE, result);
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
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(SILENT_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::path, argv[5]);
                    EXPECT_STREQ(LS_PATH, argv[6]);
                    EXPECT_STREQ(::pear::flag::command, argv[7]);
                    EXPECT_STREQ(LS_ARGV[0], argv[8]);
                    EXPECT_STREQ(LS_ARGV[1], argv[9]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        pid_t pid;
        auto result = Sut(&SILENT_LIBRARY_SESSION).posix_spawn(
                &pid,
                LS_PATH,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_env) {
        using Sut = ::ear::Executor<BrokenResolver>;
        const ::ear::LibrarySession *session_ptr = nullptr;

        pid_t pid;
        auto result = Sut(session_ptr).posix_spawnp(
                &pid,
                LS_FILE,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(FAILURE, result);
    }

    TEST_F(ExecutorTest, spawnp_fails_without_resolver) {
        using Sut = ::ear::Executor<BrokenResolver>;


        pid_t pid;
        auto result = Sut(&SILENT_LIBRARY_SESSION).posix_spawnp(
                &pid,
                LS_FILE,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(FAILURE, result);
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
                    EXPECT_STREQ(SILENT_SESSION.reporter, path);
                    EXPECT_STREQ(SILENT_SESSION.reporter, argv[0]);
                    EXPECT_STREQ(::pear::flag::destination, argv[1]);
                    EXPECT_STREQ(SILENT_SESSION.destination, argv[2]);
                    EXPECT_STREQ(::pear::flag::library, argv[3]);
                    EXPECT_STREQ(SILENT_LIBRARY_SESSION.library, argv[4]);
                    EXPECT_STREQ(::pear::flag::file, argv[5]);
                    EXPECT_STREQ(LS_FILE, argv[6]);
                    EXPECT_STREQ(::pear::flag::command, argv[7]);
                    EXPECT_STREQ(LS_ARGV[0], argv[8]);
                    EXPECT_STREQ(LS_ARGV[1], argv[9]);
                    EXPECT_EQ(LS_ENVP, envp);
                    return SUCCESS;
                };
            }
        };
        using Sut = ::ear::Executor < Validator>;

        pid_t pid;
        auto result = Sut(&SILENT_LIBRARY_SESSION).posix_spawnp(
                &pid,
                LS_FILE,
                nullptr,
                nullptr,
                LS_ARGV,
                LS_ENVP);
        EXPECT_EQ(SUCCESS, result);
    }

}