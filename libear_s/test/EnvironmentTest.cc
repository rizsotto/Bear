#include "gtest/gtest.h"

#include "../source/Environment.h"

namespace {

    class Fixture : public ::ear::Environment {
    public:
        using ::ear::Environment::get_env;
    };

    constexpr static char key[] = "this";

    TEST(get_env, finds_when_contains) {
        const char *input[] = { "that=isnot", "this=isit", nullptr };

        EXPECT_STREQ("isit", Fixture::get_env(input, key));
    }

    TEST(get_env, dont_find_when_not_conatains) {
        const char *input[] = { "these=those", nullptr };

        EXPECT_STREQ(nullptr, Fixture::get_env(input, key));
    }

    TEST(get_env, dont_find_when_prefix_long) {
        const char *input[] = { "thisisit=that", nullptr };

        EXPECT_STREQ(nullptr, Fixture::get_env(input, key));
    }

    TEST(capture_env, returns_nullptr_when_no_env) {
        char buffer[sizeof(::ear::Environment)];

        EXPECT_EQ(nullptr, ::ear::Environment::create(nullptr, buffer));
    }

    TEST(capture_env, returns_nullptr_when_missing) {
        char buffer[sizeof(::ear::Environment)];
        const char *input[] = { "this=that", nullptr };

        EXPECT_EQ(nullptr, ::ear::Environment::create(input, buffer));
    }

    TEST(capture_env, capture_correct_env_values) {
        char buffer[sizeof(::ear::Environment)];
        const char *input[] = {
                "BEAR_TARGET=/tmp/pear.random",
                "BEAR_LIBRARY=/usr/libexec/libear.so",
                "BEAR_WRAPPER=/usr/bin/pear",
                nullptr
        };

        auto result = ::ear::Environment::create(input, buffer);

        EXPECT_EQ(reinterpret_cast<::ear::Environment*>(buffer), result);
        EXPECT_STREQ("/tmp/pear.random", result->target());
        EXPECT_STREQ("/usr/libexec/libear.so", result->library());
        EXPECT_STREQ("/usr/bin/pear", result->wrapper());
    }

}