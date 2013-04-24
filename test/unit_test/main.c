// This file is distributed under MIT-LICENSE. See COPYING for details.

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
#include <json.h>

void assert_stringarray_equals(char const ** const lhs, char const ** const rhs)
{
    assert(bear_strings_length(lhs) == bear_strings_length(rhs));
    size_t const length = bear_strings_length(lhs);
    size_t i = 0;
    for (; i < length; ++i)
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

    result = bear_strings_append(result, strdup("this"));

    assert(1 == bear_strings_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == result[1]);

    result = bear_strings_append(result, strdup("that"));

    assert(2 == bear_strings_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == strcmp("that", result[1]));
    assert(0 == result[2]);

    bear_strings_release(result);
}

void test_strings_remove()
{
    char const ** result = 0;

    result = bear_strings_append(result, strdup("this"));
    result = bear_strings_append(result, strdup("and"));
    result = bear_strings_append(result, strdup("that"));

    char const * removed = result[1];
    result = bear_strings_remove(result, removed);

    assert(2 == bear_strings_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == strcmp("that", result[1]));
    assert(0 == result[2]);

    bear_strings_release(result);
    free((void *)removed);
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
    assert(bear_strings_find(input, "this"));
    assert(bear_strings_find(input, "my"));

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

    char const ** result = bear_strings_build(arg, args);

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
        "LD_PRELOAD=/tmp/lib",
        0
    };
    char const ** result = bear_strings_copy(input);
    char const * leak_guard_1 = result[1];
    char const * leak_guard_2 = result[3];

    result = bear_env_insert(result, "BEAR_OUTPUT", "/tmp/other_socket");
    result = bear_env_insert(result, "LD_PRELOAD", "/tmp/other_lib");

    char const * expected[] =
    {
        "HOME=/home/user",
        "LD_PRELOAD_NOW=what_is_this",
        "BEAR_OUTPUT=/tmp/other_socket",
        "LD_PRELOAD=/tmp/other_lib",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(result);
    free((void *)leak_guard_1);
    free((void *)leak_guard_2);
}

void test_json()
{
    char const * input_const[] =
    {
        "this",
        "is my",
        "message=\"shit\\gold\"",
        "with\tall the\rbad\nwhitespaces",
        0
    };
    char const ** input = bear_strings_copy(input_const);
    char const ** result = bear_json_escape_strings(input);

    char const * expected[] =
    {
        "this",
        "\\\"is my\\\"",
        "message=\\\"shit\\\\gold\\\"",
        "\\\"with all the bad whitespaces\\\"",
        0
    };
    assert_stringarray_equals(expected, result);

    bear_strings_release(input);
}

void assert_messages_equals(struct bear_message const * lhs,
                            struct bear_message const * rhs)
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
    struct bear_message input = { 9, 1, "exec", "/tmp", cmds };
    struct bear_message result;
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
    test_strings_remove();
    test_strings_find();
    test_strings_copy();
    test_strings_build();
    test_env_insert();
    test_json();
    test_protocol();
    return 0;
}
