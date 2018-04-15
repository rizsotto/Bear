#include "gtest/gtest.h"

#include "libear_a/Storage.h"

namespace {

    TEST(buffer, stores_does_not_crash_on_nullptr) {
        ::ear::Storage sut;
        auto result = sut.store(nullptr);

        EXPECT_EQ(nullptr, result);
    }

    TEST(buffer, stores_one_element) {
        char const *input = "hi there world";

        ::ear::Storage sut;
        auto result = sut.store(input);

        EXPECT_STREQ(input, result);
    }

    TEST(buffer, stores_multiple_element) {
        char const *input1 = "hi there world";
        char const *input2 = "how are you?";

        ::ear::Storage sut;
        sut.store(input1);
        auto result = sut.store(input2);

        EXPECT_STREQ(input2, result);
    }

}
