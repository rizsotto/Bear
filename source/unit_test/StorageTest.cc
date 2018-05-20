#include "gtest/gtest.h"

#include "libear_a/Storage.h"

namespace {

    TEST(Storage, dont_crash_on_nullptr) {
        ::ear::Storage sut;

        EXPECT_EQ(nullptr, sut.store(nullptr));
    }

    TEST(Storage, stores) {
        ::ear::Storage sut;

        const char *literal = "Hi there people";
        EXPECT_STREQ(literal, sut.store(literal));
    }

    TEST(Storage, not_same_ptr) {
        ::ear::Storage sut;

        const char *literal = "Hi there people";
        EXPECT_NE(literal, sut.store(literal));
    }

    TEST(Storage, works_multiple_times) {
        ::ear::Storage sut;

        const char *literal = "Hi there people";

        const char *result0 = sut.store(literal);
        const char *result1 = sut.store(literal);

        EXPECT_STREQ(literal, result0);
        EXPECT_STREQ(literal, result1);
    }

}