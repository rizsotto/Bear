#include "gtest/gtest.h"

#include "libear_a/Storage.h"
#include "libear_a/Session.h"
#include "libear_a/Environment.h"

namespace {

    struct Environment : public ::testing::Test {
        static constexpr size_t buffer_size = 128;

        char buffer[buffer_size];
        ::ear::Storage storage;
        ::ear::LibrarySession session;

        Environment() noexcept
                : buffer()
                , storage(buffer, buffer + buffer_size)
                , session()
        { }
    };

    TEST_F(Environment, dont_crash_on_nullptr) {
        EXPECT_EQ(nullptr, ::ear::environment::capture(session, storage, nullptr));
    }

    TEST_F(Environment, returns_nullptr_when_missing) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_EQ(nullptr, ::ear::environment::capture(session, storage, envp));
    }

    TEST_F(Environment, capture_correct_env_values) {
        const char *envp[] = {
                "EAR_DESTINATION=/tmp/pear.random",
                "EAR_LIBRARY=/usr/libexec/libear.so",
                "EAR_REPORTER=/usr/bin/pear",
                nullptr
        };

        EXPECT_EQ(&session, ::ear::environment::capture(session, storage, envp));

        EXPECT_STREQ("/tmp/pear.random", session.session.destination);
        EXPECT_STREQ("/usr/libexec/libear.so", session.library);
        EXPECT_STREQ("/usr/bin/pear", session.session.reporter);
    }

}