#include "gtest/gtest.h"

#include "Environment.h"

namespace {

    TEST(environment, empty_gets_empty_list) {
        ::pear::Environment::Builder builder(nullptr);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_EQ(nullptr, result[0]);
    }

    TEST(environment, not_empty_says_the_same) {
        const char *envp[] = {
                "THIS=that",
                nullptr
        };
        ::pear::Environment::Builder builder(envp);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("THIS=that", result[0]);
    }

    TEST(environment, reporter_inserted) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_reporter("/usr/libexec/intercept");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_REPORT_COMMAND=/usr/libexec/intercept", result[0]);
    }

    TEST(environment, destination_inserted) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_destination("/tmp/intercept");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_REPORT_DESTINATION=/tmp/intercept", result[0]);
    }

    TEST(environment, verbose_enabled) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_verbose(true);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_VERBOSE=1", result[0]);
    }

    TEST(environment, verbose_disabled) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_verbose(false);
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_EQ(nullptr, result[0]);
    }

#ifdef APPLE
#else
    TEST(environment, empty_library) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so", result[1]);
    }

    TEST(environment, library_already_there) {
        const char *envp[] = {
                "LD_PRELOAD=/usr/libexec/libexec.so",
                nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so", result[1]);
    }

    TEST(environment, library_already_with_another) {
        const char *envp[] = {
                "LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so",
                nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so", result[1]);
    }

    TEST(environment, another_libray_is_there) {
        const char *envp[] = {
                "LD_PRELOAD=/usr/libexec/libio.so",
                nullptr
        };
        ::pear::Environment::Builder builder(envp);
        builder.add_library("/usr/libexec/libexec.so");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("INTERCEPT_LIBRARY=/usr/libexec/libexec.so", result[0]);
        EXPECT_STREQ("LD_PRELOAD=/usr/libexec/libexec.so:/usr/libexec/libio.so", result[1]);
    }
#endif

    TEST(environment, cc_wrapper_inserted) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_cc_compiler("cc", "/usr/libexec/intercept-cc");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("CC=/usr/libexec/intercept-cc", result[0]);
        EXPECT_STREQ("INTERCEPT_SESSION_CC=cc", result[1]);
    }

    TEST(environment, cxx_wrapper_inserted) {
        ::pear::Environment::Builder builder(nullptr);
        builder.add_cxx_compiler("c++", "/usr/libexec/intercept-c++");
        auto sut = builder.build();
        auto result = sut->data();

        EXPECT_NE(nullptr, result);
        EXPECT_STREQ("CXX=/usr/libexec/intercept-c++", result[0]);
        EXPECT_STREQ("INTERCEPT_SESSION_CXX=c++", result[1]);
    }

}