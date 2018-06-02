#include "gtest/gtest.h"

#include "../Storage.h"
#include "../Interface.h"
#include "../Environment.h"

namespace {

    struct Environment : public ::testing::Test {
        ::ear::LibrarySession librarySession;
        ::ear::WrapperSession wrapperSession;

        Environment() noexcept
                : librarySession()
                , wrapperSession()
        { }
    };

    TEST_F(Environment, dont_crash_on_nullptr_4_library) {
        EXPECT_EQ(nullptr, ::ear::environment::capture(librarySession, nullptr));
    }

    TEST_F(Environment, dont_crash_on_nullptr_4_wrapper) {
        EXPECT_EQ(nullptr, ::ear::environment::capture(wrapperSession, nullptr));
    }

    TEST_F(Environment, returns_nullptr_when_missing_4_library) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_EQ(nullptr, ::ear::environment::capture(librarySession, envp));
    }

    TEST_F(Environment, returns_nullptr_when_missing_4_wrapper) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        EXPECT_EQ(nullptr, ::ear::environment::capture(wrapperSession, envp));
    }

    TEST_F(Environment, capture_4_libray) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                nullptr
        };

        EXPECT_EQ(&librarySession, ::ear::environment::capture(librarySession, envp));

        EXPECT_STREQ("/tmp/intercept.random", librarySession.session.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", librarySession.library);
        EXPECT_STREQ("/usr/bin/intercept", librarySession.session.reporter);
        EXPECT_EQ(false, librarySession.session.verbose);
    }

    TEST_F(Environment, capture_4_wrapper) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_SESSION_CC=/usr/bin/cc",
                "INTERCEPT_SESSION_CXX=/usr/bin/c++",
                nullptr
        };

        EXPECT_EQ(&wrapperSession, ::ear::environment::capture(wrapperSession, envp));

        EXPECT_STREQ("/tmp/intercept.random", wrapperSession.session.destination);
        EXPECT_STREQ("/usr/bin/intercept", wrapperSession.session.reporter);
        EXPECT_EQ(false, wrapperSession.session.verbose);
        EXPECT_STREQ("/usr/bin/cc", wrapperSession.cc);
        EXPECT_STREQ("/usr/bin/c++", wrapperSession.cxx);
    }

    TEST_F(Environment, capture_verbose_4_library) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_SESSION_LIBRARY=/usr/libexec/libexec.so",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                nullptr
        };

        EXPECT_EQ(&librarySession, ::ear::environment::capture(librarySession, envp));

        EXPECT_STREQ("/tmp/intercept.random", librarySession.session.destination);
        EXPECT_STREQ("/usr/libexec/libexec.so", librarySession.library);
        EXPECT_STREQ("/usr/bin/intercept", librarySession.session.reporter);
        EXPECT_EQ(true, librarySession.session.verbose);
    }

    TEST_F(Environment, capture_verbose_4_wrapper) {
        const char *envp[] = {
                "INTERCEPT_REPORT_DESTINATION=/tmp/intercept.random",
                "INTERCEPT_REPORT_COMMAND=/usr/bin/intercept",
                "INTERCEPT_VERBOSE=true",
                "INTERCEPT_SESSION_CC=/usr/bin/cc",
                "INTERCEPT_SESSION_CXX=/usr/bin/c++",
                nullptr
        };

        EXPECT_EQ(&wrapperSession, ::ear::environment::capture(wrapperSession, envp));

        EXPECT_STREQ("/tmp/intercept.random", wrapperSession.session.destination);
        EXPECT_STREQ("/usr/bin/intercept", wrapperSession.session.reporter);
        EXPECT_EQ(true, wrapperSession.session.verbose);
        EXPECT_STREQ("/usr/bin/cc", wrapperSession.cc);
        EXPECT_STREQ("/usr/bin/c++", wrapperSession.cxx);
    }

}