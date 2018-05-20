#include "gtest/gtest.h"

#include "libear_a/Storage.h"
#include "libear_a/Session.h"
#include "libear_a/Environment.h"

namespace {

    TEST(environment, dont_crash_on_nullptr) {
        ::ear::Storage storage;
        ::ear::LibrarySession session;

        EXPECT_EQ(nullptr, ::ear::environment::capture(session, storage, nullptr));
    }

    TEST(environment, returns_nullptr_when_missing) {
        const char *envp[] = { "this=is", "these=are", nullptr };

        ::ear::Storage storage;
        ::ear::LibrarySession session;

        EXPECT_EQ(nullptr, ::ear::environment::capture(session, storage, envp));
    }

    TEST(environment, capture_correct_env_values) {
        const char *envp[] = {
                "EAR_DESTINATION=/tmp/pear.random",
                "EAR_LIBRARY=/usr/libexec/libear.so",
                "EAR_REPORTER=/usr/bin/pear",
                nullptr
        };

        ::ear::Storage storage;
        ::ear::LibrarySession session;

        EXPECT_EQ(&session, ::ear::environment::capture(session, storage, envp));

        EXPECT_STREQ("/tmp/pear.random", session.destination);
        EXPECT_STREQ("/usr/libexec/libear.so", session.library);
        EXPECT_STREQ("/usr/bin/pear", session.reporter);
    }

}