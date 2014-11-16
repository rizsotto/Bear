/*  Copyright (C) 2012-2014 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#undef NDEBUG
#include <assert.h>

#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <fcntl.h>

#include <stringarray.h>
#include <environ.h>
#include <protocol.h>
#include <stringtransform.h>

void assert_stringarray_equals(char const ** const lhs, char const ** const rhs)
{
    assert(bear_strings_length(lhs) == bear_strings_length(rhs));
    size_t const length = bear_strings_length(lhs);
    for (size_t i = 0; i < length; ++i)
    {
        assert(0 == strcmp(lhs[i], rhs[i]));
    }
}

void test_strings_length()
{
    char const * input[] =
    {
        "this",
        "is",
        "my",
        "message",
        0
    };
    assert(4 == bear_strings_length(input));
    assert(2 == bear_strings_length(input + 2));
    assert(0 == bear_strings_length(0));
}

void test_strings_fold()
{
    char const * input[] =
    {
        "this",
        "is",
        "my",
        "message",
        0
    };
    char const * const expected = "this\x1fis\x1fmy\x1fmessage";
    char const * const result = bear_strings_fold(input, '\x1f');

    assert((0 == strcmp(expected, result)) && "bear_strings_fold failed");

    free((void *)result);
}

void test_strings_append()
{
    char const ** result = 0;

    char const * const this = "this";
    char const * const that = "that";
    result = bear_strings_append(result, this);

    assert(1 == bear_strings_length(result));
    assert(this == result[0]);
    assert(0 == result[1]);

    result = bear_strings_append(result, that);

    assert(2 == bear_strings_length(result));
    assert(this == result[0]);
    assert(that == result[1]);
    assert(0 == result[2]);

    free((void *)result);
}

void test_strings_find()
{
    char const * input[] =
    {
        "this",
        "is",
        "my",
        "message",
        0
    };
    assert(input[0] == bear_strings_find(input, "this"));
    assert(input[2] == bear_strings_find(input, "my"));

    assert(0 == bear_strings_find(input, "th"));
    assert(0 == bear_strings_find(input, "messa"));
}

void test_strings_copy()
{
    char const * input[] =
    {
        "this",
        "is",
        "my",
        "message",
        0
    };
    char const ** result = bear_strings_copy(input);

    assert_stringarray_equals(input, result);
    assert(input != result);

    bear_strings_release(result);
}

char const ** bear_strings_build_stdarg_driver(char const * arg, ...)
{
    va_list args;
    va_start(args, arg);

    char const ** result = bear_strings_build(arg, &args);

    va_end(args);
    return result;
}

void test_strings_build()
{
    char const ** result = bear_strings_build_stdarg_driver("this", "is", "my", "message", 0);

    char const * expected[] =
    {
        "this",
        "is",
        "my",
        "message",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(result);
}

void test_env_insert()
{
    char const * input[] =
    {
        "HOME=/home/user",
        "BEAR_OUTPUT=/tmp/socket",
        "LD_PRELOAD_NOW=what_is_this",
        0
    };
    char const ** result = bear_strings_copy(input);

    setenv("BEAR_OUTPUT", "/tmp/other_socket", 1);
    setenv("LD_PRELOAD", "/tmp/other_lib", 1);
    setenv("HOME", "/home/user", 1);
    result = bear_update_environ(result, "HOME");
    result = bear_update_environ(result, "BEAR_OUTPUT");
    result = bear_update_environ(result, "LD_PRELOAD");

    char const * expected[] =
    {
        "HOME=/home/user",
        "BEAR_OUTPUT=/tmp/other_socket",
        "LD_PRELOAD_NOW=what_is_this",
        "LD_PRELOAD=/tmp/other_lib",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(result);
}

void test_json_escape()
{
    char const * input[] =
    {
        "no escaping for this one",
        "symbolic: BS \b FF \f LF \n CR \r HT \t slash \\ quote \"",
        "numeric: BEL \a VT \v ESC \x1b",
        "mix: \a \b c",
        0
    };
    char const ** result = bear_strings_copy(input);
    bear_strings_transform(result, bear_string_json_escape);

    char const * expected[] =
    {
        "no escaping for this one",
        "symbolic: BS \\b FF \\f LF \\n CR \\r HT \\t slash \\\\ quote \\\"",
        "numeric: BEL \\u0007 VT \\u000b ESC \\u001b",
        "mix: \\u0007 \\b c",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(result);
}

void test_shell_escape()
{
    char const * input[] =
    {
        "$no_escaping(\r)",
        "escaped:\"\\",
        "quoted: \t\n",
        "quoted\\and escaped",
        0
    };
    char const ** result = bear_strings_copy(input);
    bear_strings_transform(result, bear_string_shell_escape);

    char const * expected[] =
    {
        "$no_escaping(\r)",
        "escaped:\\\"\\\\",
        "\"quoted: \t\n\"",
        "\"quoted\\\\and escaped\"",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(result);
}

void assert_messages_equals(bear_message_t const * lhs,
                            bear_message_t const * rhs)
{
    assert(lhs->pid == rhs->pid);
    assert(lhs->ppid == rhs->ppid);
    assert(0 == strcmp(lhs->fun, rhs->fun));
    assert(0 == strcmp(lhs->cwd, rhs->cwd));
    assert_stringarray_equals(lhs->cmd, rhs->cmd);
}

void test_protocol()
{
    char const * cmds[] =
    {
        "this",
        "that",
        0
    };
    bear_message_t input = { 9, 1, "exec", "/tmp", cmds };
    bear_message_t result;
    {
        int fds[2];
        pipe(fds);

        bear_write_message(fds[1], &input);
        bear_read_message(fds[0], &result);
    }
    assert_messages_equals(&input, &result);
    bear_free_message(&result);
}


int main()
{
    test_strings_length();
    test_strings_fold();
    test_strings_append();
    test_strings_find();
    test_strings_copy();
    test_strings_build();
    test_env_insert();
    test_json_escape();
    test_shell_escape();
    test_protocol();
    return 0;
}
