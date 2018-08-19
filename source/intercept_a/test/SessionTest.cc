#include "gtest/gtest.h"

#include "intercept_a/Session.h"

namespace {

    TEST(session, parse_empty_fails) {
        const char *argv[] = { "program", nullptr };

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(1, const_cast<char **>(argv));
        ::pear::SessionPtr const expected = ::pear::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_help_fails) {
        const char *argv[] = { "program", "--help", nullptr };

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(1, const_cast<char **>(argv));
        ::pear::SessionPtr const expected = ::pear::SessionPtr(nullptr);

        ASSERT_EQ(expected, result.get_or_else(expected));
    }

    TEST(session, parse_library_success) {
        const char *argv[] = { "program",
                               ::pear::flag::library, "/install/path/libexec.so",
                               ::pear::flag::destination, "/tmp/destination",
                               ::pear::flag::verbose,
                               ::pear::flag::command, "ls", "-l", "-a",
                               nullptr };

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(1, const_cast<char **>(argv));
        ::pear::SessionPtr const dummy = ::pear::SessionPtr(nullptr);
        ASSERT_NE(dummy, result.get_or_else(dummy));
        auto session_result = (::pear::LibrarySession const *)result.get_or_else(dummy).get();

        ASSERT_STREQ(argv[0], session_result->context_.reporter);
        ASSERT_STREQ(argv[4], session_result->context_.destination);
        ASSERT_EQ(true, session_result->context_.verbose);

        ASSERT_EQ(argv + 7, session_result->execution_.command);
        ASSERT_EQ(nullptr, session_result->execution_.search_path);
        ASSERT_EQ(nullptr, session_result->execution_.file);

        ASSERT_EQ(argv[2], session_result->library);
    }

    TEST(session, parse_wrapper_success) {
        const char *argv[] = { "program",
                               ::pear::flag::wrapper_cc, "cc", "/install/path/wrapper-cc",
                               ::pear::flag::wrapper_cxx, "c++", "/install/path/wrapper-c++",
                               ::pear::flag::destination, "/tmp/destination",
                               ::pear::flag::file, "/bin/ls",
                               ::pear::flag::command, "ls", "-l", "-a",
                               nullptr };

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(1, const_cast<char **>(argv));
        ::pear::SessionPtr const dummy = ::pear::SessionPtr(nullptr);
        ASSERT_NE(dummy, result.get_or_else(dummy));
        auto session_result = (::pear::WrapperSession const *)result.get_or_else(dummy).get();

        ASSERT_STREQ(argv[0], session_result->context_.reporter);
        ASSERT_STREQ(argv[8], session_result->context_.destination);
        ASSERT_EQ(false, session_result->context_.verbose);

        ASSERT_EQ(argv + 12, session_result->execution_.command);
        ASSERT_EQ(nullptr, session_result->execution_.search_path);
        ASSERT_STREQ(argv[10], session_result->execution_.file);

        ASSERT_STREQ(argv[2], session_result->cc);
        ASSERT_STREQ(argv[3], session_result->cc_wrapper);
        ASSERT_STREQ(argv[5], session_result->cxx);
        ASSERT_STREQ(argv[6], session_result->cxx_wrapper);
    }

    TEST(session, parse_simple_success) {
        const char *argv[] = { "program",
                               ::pear::flag::destination, "/tmp/destination",
                               ::pear::flag::search_path, "/bin:/usr/bin",
                               ::pear::flag::command, "ls", "-l", "-a",
                               nullptr };

        ::pear::Result<::pear::SessionPtr> const result = ::pear::parse(1, const_cast<char **>(argv));
        ::pear::SessionPtr const dummy = ::pear::SessionPtr(nullptr);
        ASSERT_NE(dummy, result.get_or_else(dummy));
        auto session_result = result.get_or_else(dummy).get();

        ASSERT_STREQ(argv[0], session_result->context_.reporter);
        ASSERT_STREQ(argv[2], session_result->context_.destination);
        ASSERT_EQ(false, session_result->context_.verbose);

        ASSERT_EQ(argv + 6, session_result->execution_.command);
        ASSERT_STREQ(argv[4], session_result->execution_.search_path);
        ASSERT_EQ(nullptr, session_result->execution_.file);
    }
}