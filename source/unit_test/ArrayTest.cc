#include "gtest/gtest.h"

#include "../libear_a/Array.h"

namespace {

    TEST(array_end, dont_crash_on_nullptr) {
        const char **input = nullptr;

        EXPECT_EQ(nullptr, ::ear::array::end(input));
    }

    TEST(array_end, dont_crash_on_empty) {
        const char *input[] = { nullptr };

        EXPECT_EQ(&input[0], ::ear::array::end(input));
    }

    TEST(array_end, finds_the_last_one) {
        const char *input0 = "this";
        const char *input1 = "that";
        const char *input[] = { input0, input1, 0 };

        EXPECT_EQ(&input[2], ::ear::array::end(input));
    }

    TEST(array_length, dont_crash_on_nullptr) {
        const char **input = nullptr;

        EXPECT_EQ(0, ::ear::array::length(input));
    }

    TEST(array_length, dont_crash_on_empty) {
        const char *input[] = { nullptr };

        EXPECT_EQ(0, ::ear::array::length(input));
    }

    TEST(array_length, finds_the_last_one) {
        const char *input0 = "this";
        const char *input1 = "that";
        const char *input[] = { input0, input1, 0 };

        EXPECT_EQ(2, ::ear::array::length(input));
    }

    TEST(array_copy, does_copy_elements_over) {
        const char src[5] = {'o', 't', 't', 'o', '\0'};
        char dst[8] = {};

        auto result = ::ear::array::copy(src, src + 5, dst, dst + 8);
        EXPECT_TRUE(result != nullptr);
        EXPECT_STREQ(src, dst);
    }

    TEST(array_copy, stops_when_short) {
        const char src[5] = {'o', 't', 't', 'o', '\0'};
        char dst[8] = {};

        auto result = ::ear::array::copy(src, src + 5, dst, dst + 3);
        EXPECT_EQ(nullptr, result);
    }

}
