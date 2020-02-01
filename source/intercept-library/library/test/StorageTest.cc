#include "gtest/gtest.h"

#include "Storage.h"

namespace {

    TEST(Storage, dont_crash_on_nullptr) {
        char buffer[64];
        ear::Storage sut(buffer, buffer + 64);

        EXPECT_EQ(nullptr, sut.store(nullptr));
    }

    TEST(Storage, stores) {
        char buffer[64];
        ear::Storage sut(buffer, buffer + 64);

        const char *literal = "Hi there people";
        EXPECT_STREQ(literal, sut.store(literal));
    }

    TEST(Storage, not_same_ptr) {
        char buffer[64];
        ear::Storage sut(buffer, buffer + 64);

        const char *literal = "Hi there people";
        EXPECT_NE(literal, sut.store(literal));
    }

    TEST(Storage, works_multiple_times) {
        char buffer[64];
        ear::Storage sut(buffer, buffer + 64);

        const char *literal0 = "Hi there people";
        const char *literal1 = "Hallo Leute";

        const char *result0 = sut.store(literal0);
        const char *result1 = sut.store(literal1);

        EXPECT_STREQ(literal0, result0);
        EXPECT_STREQ(literal1, result1);
    }

    TEST(Storage, handles_size_issue) {
        char buffer[8];
        ear::Storage sut(buffer, buffer + 8);

        const char *literal = "Hi there people";

        EXPECT_EQ(nullptr, sut.store(literal));
    }

}
