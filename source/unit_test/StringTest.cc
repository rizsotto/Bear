#include "gtest/gtest.h"

#include "../libear_a/String.h"

namespace {

    TEST(string_end, dont_crash_on_nullptr) {
        EXPECT_EQ(nullptr, ::ear::string::end(nullptr));
    }

    TEST(string_end, dont_crash_on_empty) {
        const char *input = "";

        EXPECT_EQ(input, ::ear::string::end(input));
    }

    TEST(string_end, finds_the_last_one) {
        const char input[] = "this";
        const size_t input_size = sizeof(input);

        EXPECT_EQ(&input[input_size - 1], ::ear::string::end(input));
    }

    TEST(string_length, dont_crash_on_nullptr) {
        EXPECT_EQ(0, ::ear::string::end(nullptr));
    }

    TEST(string_length, dont_crash_on_empty) {
        EXPECT_EQ(0, ::ear::string::length(""));
    }

    TEST(string_length, finds_the_last_one) {
        const char input[] = "this";
        const size_t input_size = sizeof(input);

        EXPECT_EQ(input_size - 1, ::ear::string::length(input));
    }

    TEST(string_equal, dont_crash_on_nullptr) {
        EXPECT_TRUE(::ear::string::equal(nullptr, nullptr, 0));
    }

    TEST(string_equal, dont_crash_on_empty) {
        EXPECT_TRUE(::ear::string::equal("", "this", 0));
    }

    TEST(string_equal, finds_prefixes_equal) {
        EXPECT_TRUE(::ear::string::equal("this", "this", 2));
        EXPECT_TRUE(::ear::string::equal("this", "this", 4));
        EXPECT_TRUE(::ear::string::equal("that", "this", 2));
        EXPECT_TRUE(::ear::string::equal("th", "this", 2));
    }

    TEST(string_equal, rejects_non_equals) {
        EXPECT_FALSE(::ear::string::equal("this", "that", 4));
    }

    TEST(string_equal, rejects_when_shorter) {
        EXPECT_FALSE(::ear::string::equal("this", "th", 4));
    }

    TEST(string_constructor, copy_content) {
        const char input[] = "Lorem ipsum dolor sit amet";
        EXPECT_STREQ(input, ::ear::String<128>(input).begin());
    }

    TEST(string_constructor, null_for_long_content) {
        const char input[] = "Lorem ipsum dolor sit amet";
        EXPECT_STREQ("", ::ear::String<16>(input).begin());
    }

}