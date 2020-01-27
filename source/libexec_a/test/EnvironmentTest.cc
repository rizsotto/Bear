#include "gtest/gtest.h"

#include "../Storage.h"
#include "../Interface.h"
#include "../Environment.h"

namespace {

    TEST(Environment, dont_crash_on_nullptr_4_library) {
        EXPECT_FALSE(::ear::environment::libray_session(nullptr).is_valid());
    }

    TEST(Environment, dont_crash_on_nullptr_4_wrapper) {
        EXPECT_FALSE(::ear::environment::wrapper_session(nullptr).is_valid());
    }

    TEST(Environment, returns_nullptr_when_missing_4_library) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_FALSE(::ear::environment::libray_session(envp).is_valid());
    }

    TEST(Environment, returns_nullptr_when_missing_4_wrapper) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_FALSE(::ear::environment::wrapper_session(envp).is_valid());
    }

    TEST(Environment, capture_4_libray) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        const auto result = ::ear::environment::libray_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.context.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.context.reporter);
        EXPECT_EQ(false, result.context.verbose);
    }

    TEST(Environment, capture_4_wrapper) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_SESSION_CC=/usr/bin/cc",
                "INTERCEPT_SESSION_CXX=/usr/bin/c++",
                nullptr
        };

        const auto result = ::ear::environment::wrapper_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.context.destination);
        EXPECT_STREQ("/usr/bin/intercept", result.context.reporter);
        EXPECT_EQ(false, result.context.verbose);
        EXPECT_STREQ("/usr/bin/cc", result.cc);
        EXPECT_STREQ("/usr/bin/c++", result.cxx);
    }

    TEST(Environment, capture_verbose_4_library) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                nullptr
        };

        const auto result = ::ear::environment::libray_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.context.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", result.library);
        EXPECT_STREQ("/usr/bin/intercept", result.context.reporter);
        EXPECT_EQ(true, result.context.verbose);
    }

    TEST(Environment, capture_verbose_4_wrapper) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                "INTERCEPT_SESSION_CC=/usr/bin/cc",
                "INTERCEPT_SESSION_CXX=/usr/bin/c++",
                nullptr
        };

        const auto result = ::ear::environment::wrapper_session(envp);

        EXPECT_STREQ("/tmp/intercept.random", result.context.destination);
        EXPECT_STREQ("/usr/bin/intercept", result.context.reporter);
        EXPECT_EQ(true, result.context.verbose);
        EXPECT_STREQ("/usr/bin/cc", result.cc);
        EXPECT_STREQ("/usr/bin/c++", result.cxx);
    }

}