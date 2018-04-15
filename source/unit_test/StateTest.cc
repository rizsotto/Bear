#include "gtest/gtest.h"

#include "libear_a/State.h"

namespace {

    class Fixture : public ::ear::State {
    public:
        using ::ear::State::get_env;
        using ::ear::State::create;
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
        char buffer[sizeof(::ear::State)];

        EXPECT_EQ(nullptr, Fixture::create(nullptr, buffer));
    }

    TEST(capture_env, returns_nullptr_when_missing) {
        char buffer[sizeof(::ear::State)];
        const char *input[] = { "this=that", nullptr };

        EXPECT_EQ(nullptr, Fixture::create(input, buffer));
    }

    TEST(capture_env, capture_correct_env_values) {
        char buffer[sizeof(::ear::State)];
        const char *input[] = {
                "EAR_DESTINATION=/tmp/pear.random",
                "EAR_LIBRARY=/usr/libexec/libear.so",
                "EAR_REPORTER=/usr/bin/pear",
                nullptr
        };

        auto sut = Fixture::create(input, buffer);
        EXPECT_EQ(reinterpret_cast<::ear::State*>(buffer), sut);

        auto result = sut->get_input();
        EXPECT_STREQ("/tmp/pear.random", result.session.destination);
        EXPECT_STREQ("/usr/libexec/libear.so", result.library);
        EXPECT_STREQ("/usr/bin/pear", result.session.reporter);
    }

}