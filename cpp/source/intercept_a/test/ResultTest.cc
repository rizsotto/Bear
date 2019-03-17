#include "gtest/gtest.h"

#include "intercept_a//Result.h"

namespace {

    using Error = const char *;
    using namespace pear;

    TEST(result, get_or_else_on_success) {
        EXPECT_EQ(2,
                  (Result<int, Error>(Ok(2))
                          .get_or_else(8))
        );
        EXPECT_EQ('c',
                  (Result<char, Error>(Ok('c'))
                          .get_or_else('+'))
        );
    }

    TEST(result, get_or_else_on_failure) {
        EXPECT_EQ(8,
                  (Result<int, Error>(Err("problem"))
                          .get_or_else(8))
        );
        EXPECT_EQ('+',
                  (Result<char, Error>(Err("problem"))
                          .get_or_else('+'))
        );
    }

    TEST(result, map_on_success) {
        EXPECT_EQ(4,
                  (Result<int, Error>(Ok(2))
                          .map<int>([](auto &in) {
                              return in * 2;
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ(2.5f,
                  (Result<int, Error>(Ok(2))
                          .map<float>([](auto &in) {
                              return in + 0.5f;
                          })
                          .get_or_else(8.0f))
        );
        EXPECT_EQ('d',
                  (Result<char, Error>(Ok('c'))
                          .map<int>([](auto &in) {
                              return in + 1;
                          })
                          .get_or_else(42))
        );
    }

    TEST(result, map_on_failure) {
        EXPECT_EQ(8,
                  (Result<int, Error>(Err("problem"))
                          .map<int>([](auto &in) {
                              return in * 2;
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ('+',
                  (Result<char, Error>(Err("problem"))
                          .map<char>([](const char &in) {
                              return char(in + 1);
                          })
                          .get_or_else('+'))
        );
    }

    TEST(result, bind_on_success) {
        EXPECT_EQ(2,
                  (Result<int, Error>(Ok(1))
                          .bind<int>([](auto &in) {
                              return Ok(in * 2);
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ('d',
                  (Result<char, Error>(Ok('c'))
                          .bind<char>([](auto &in) {
                              return Ok(char(in + 1));
                          })
                          .get_or_else('+'))
        );
        EXPECT_EQ(8,
                  (Result<int, Error>(Ok(1))
                          .bind<int>([](auto &in) {
                              return Err("problem");
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ('+',
                  (Result<char, Error>(Ok('c'))
                          .bind<char>([](auto &in) {
                              return Err("problem");
                          })
                          .get_or_else('+'))
        );
    }

    TEST(result, bind_on_failure) {
        EXPECT_EQ(8,
                  (Result<int, Error>(Err("problem"))
                          .bind<int>([](auto &in) {
                              return Ok(in * 2);
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ('+',
                  (Result<char, Error>(Err("problem"))
                          .bind<char>([](auto &in) {
                              return Ok(char(in + 1));
                          })
                          .get_or_else('+'))
        );
        EXPECT_EQ(8,
                  (Result<int, Error>(Err("problem"))
                          .bind<int>([](auto &in) {
                              return Err("another problem");
                          })
                          .get_or_else(8))
        );
        EXPECT_EQ('+',
                  (Result<char, Error>(Err("problem"))
                          .bind<char>([](auto &in) {
                              return Err("another problem");
                          })
                          .get_or_else('+'))
        );
    }

    TEST(result, handle_with_on_success) {
        char const *result = "expected";

        Result<int, Error>(Ok(1))
                .handle_with([&result](char const *in) {
                    result = in;
                });
        EXPECT_STREQ("expected", result);
    }

    TEST(result, handle_with_on_failure) {
        char const *result = "expected";

        Result<int, Error>(Err("problem"))
                .handle_with([&result](char const *in) {
                    result = in;
                });
        EXPECT_STREQ("problem", result);
    }

}