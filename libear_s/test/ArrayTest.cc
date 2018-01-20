#include "gtest/gtest.h"

#include "../source/Array.h"

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

}
