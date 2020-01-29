#include "gtest/gtest.h"

#include "../Array.h"

namespace {

    TEST(array_end, dont_crash_on_nullptr) {
        const char **input = nullptr;

        EXPECT_EQ(nullptr, ear::array::end(input));
    }

    TEST(array_end, dont_crash_on_empty) {
        const char *input[] = { nullptr };

        EXPECT_EQ(&input[0], ear::array::end(input));
    }

    TEST(array_end, finds_the_last_one) {
        const char *input0 = "this";
        const char *input1 = "that";
        const char *input[] = { input0, input1, 0 };

        EXPECT_EQ(&input[2], ear::array::end(input));
    }

    TEST(array_length, dont_crash_on_nullptr) {
        const char **input = nullptr;

        EXPECT_EQ(0, ear::array::length(input));
    }

    TEST(array_length, dont_crash_on_empty) {
        const char *input[] = { nullptr };

        EXPECT_EQ(0, ear::array::length(input));
    }

    TEST(array_length, finds_the_last_one) {
        const char *input0 = "this";
        const char *input1 = "that";
        const char *input[] = { input0, input1, 0 };

        EXPECT_EQ(2, ear::array::length(input));
    }

    TEST(array_copy, works_with_zero_length_input) {
        const char src[5] = "";
        char dst[8] = {};

        auto result = ear::array::copy(src, src, dst, dst + 8);
        EXPECT_EQ(dst, result);
    }

    TEST(array_copy, does_copy_elements_over) {
        const char src[5] = "this";
        char dst[8] = {};

        auto result = ear::array::copy(src, src + 5, dst, dst + 8);
        EXPECT_NE(result, nullptr);
        EXPECT_EQ((dst + 5), result);
        EXPECT_STREQ(src, dst);
    }

    TEST(array_copy, does_copy_elements_into_same_size) {
        const char src[5] = "this";
        char dst[5] = {};

        auto result = ear::array::copy(src, src + 5, dst, dst + 5);
        EXPECT_NE(result, nullptr);
        EXPECT_EQ((dst + 5), result);
        EXPECT_STREQ(src, dst);
    }

    TEST(array_copy, stops_when_short) {
        const char src[5] = "this";
        char dst[8] = {};

        auto result = ear::array::copy(src, src + 5, dst, dst + 3);
        EXPECT_EQ(nullptr, result);
    }

}
