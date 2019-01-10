#include "gtest/gtest.h"

#include "../Session.h"
#include "../Environment.h"

namespace {

    TEST(Environment, dont_crash_on_nullptr) {
        const auto result = ::ear::environment::capture_session(nullptr);
        ASSERT_TRUE(result.is_not_valid());
    }

    TEST(Environment, capture_on_empty) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        const auto result = ::ear::environment::capture_session(envp);
        ASSERT_TRUE(result.is_not_valid());
    }

    TEST(Environment, capture_silent) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        const auto result = ::ear::environment::capture_session(envp);
        ASSERT_FALSE(result.is_not_valid());

        EXPECT_STREQ("/tmp/intercept.random", result.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.reporter);
        EXPECT_EQ(false, result.verbose);
    }

    TEST(Environment, capture_verbose) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                nullptr
        };

        const auto result = ::ear::environment::capture_session(envp);
        ASSERT_FALSE(result.is_not_valid());

        EXPECT_STREQ("/tmp/intercept.random", result.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.reporter);
        EXPECT_EQ(true, result.verbose);
    }

}