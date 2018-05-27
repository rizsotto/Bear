#include "gtest/gtest.h"

#include "../Storage.h"
#include "../Session.h"
#include "../Environment.h"

namespace {

    struct Environment : public ::testing::Test {
        ::ear::LibrarySession session;

        Environment() noexcept
                : session()
        { }
    };

    TEST_F(Environment, dont_crash_on_nullptr) {
        EXPECT_EQ(nullptr, ::ear::environment::capture(session, nullptr));
    }

    TEST_F(Environment, returns_nullptr_when_missing) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_EQ(nullptr, ::ear::environment::capture(session, envp));
    }

    TEST_F(Environment, capture_correct_env_values) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        EXPECT_EQ(&session, ::ear::environment::capture(session, envp));

        EXPECT_STREQ("/tmp/intercept.random", session.session.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", session.library);
        EXPECT_STREQ("/usr/bin/intercept", session.session.reporter);
    }

}