#include "gtest/gtest.h"

#include "../Storage.h"
#include "../Interface.h"
#include "../Environment.h"

namespace {

    TEST(Environment, dont_crash_on_nullptr_4_library) {
        EXPECT_FALSE(::ear::environment::capture_session(nullptr).is_valid());
    }

    TEST(Environment, returns_nullptr_when_missing_4_library) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_FALSE(::ear::environment::capture_session(envp).is_valid());
    }

    TEST(Environment, capture_4_libray) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        const auto result = ::ear::environment::capture_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.reporter);
        EXPECT_EQ(false, result.verbose);
    }

    TEST(Environment, capture_verbose_4_library) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                nullptr
        };

        const auto result = ::ear::environment::capture_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.reporter);
        EXPECT_EQ(true, result.verbose);
    }

}