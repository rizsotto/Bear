// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <assert.h>

#include <malloc.h>
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

void assert_stringarray_equals(Strings const lhs, Strings const rhs) {
    assert(sa_length(lhs) == sa_length(rhs));
    size_t const length = sa_length(lhs);
    int i = 0;
    for (; i < length; ++i) {
        assert(0 == strcmp(lhs[i], rhs[i]));
    }
}

void test_sa_length() {
    char const * input[] =
        { "this"
        , "is"
        , "my"
        , "message"
        , 0
        };
    assert(4 == sa_length(input));
    assert(2 == sa_length(input + 2));
    assert(0 == sa_length(0));
}

void test_sa_fold() {
    char const * input[] =
        { "this"
        , "is"
        , "my"
        , "message"
        , 0
        };
    char const * const expected = "this\x1fis\x1fmy\x1fmessage";
    char const * const result = sa_fold(input, '\x1f');

    assert((0 == strcmp(expected, result)) && "sa_fold failed");

    free((void *)result);
}

void test_sa_append() {
    Strings result = 0;

    result = sa_append(result, strdup("this"));

    assert(1 == sa_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == result[1]);

    result = sa_append(result, strdup("that"));

    assert(2 == sa_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == strcmp("that", result[1]));
    assert(0 == result[2]);

    sa_release(result);
}

void test_sa_remove() {
    Strings result = 0;

    result = sa_append(result, strdup("this"));
    result = sa_append(result, strdup("and"));
    result = sa_append(result, strdup("that"));

    result = sa_remove(result, result[1]);

    assert(2 == sa_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == strcmp("that", result[1]));
    assert(0 == result[2]);

    sa_release(result);
}

void test_sa_find() {
    char const * input[] =
        { "this"
        , "is"
        , "my"
        , "message"
        , 0
        };
    assert(sa_find(input, "this"));
    assert(sa_find(input, "my"));

    assert(0 == sa_find(input, "th"));
    assert(0 == sa_find(input, "messa"));
}

void test_sa_copy() {
    char const * input[] =
        { "this"
        , "is"
        , "my"
        , "message"
        , 0
        };
    Strings result = sa_copy(input);

    assert(4 == sa_length(result));
    assert(0 == strcmp("this",      result[0]));
    assert(0 == strcmp("is",        result[1]));
    assert(0 == strcmp("my",        result[2]));
    assert(0 == strcmp("message",   result[3]));
    assert(0 == result[4]);

    sa_release(result);
}

Strings sa_build_stdarg_driver(char const * arg, ...) {
    va_list args;
    va_start(args, arg);

    Strings result = sa_build(arg, args);

    va_end(args);
    return result;
}

void test_sa_build() {
    Strings result = sa_build_stdarg_driver("this", "is", "my", "message", 0);

    assert(4 == sa_length(result));
    assert(0 == strcmp("this",      result[0]));
    assert(0 == strcmp("is",        result[1]));
    assert(0 == strcmp("my",        result[2]));
    assert(0 == strcmp("message",   result[3]));
    assert(0 == result[4]);

    sa_release(result);
}

void test_env_insert() {
    char const * input[] =
        { "HOME=/home/user"
        , "BEAR_OUTPUT=/tmp/socket"
        , "LD_PRELOAD_NOW=what_is_this"
        , "LD_PRELOAD=/tmp/lib"
        , 0
        };
    Strings result = sa_copy(input);

    result = bear_env_insert(result, "BEAR_OUTPUT", "/tmp/other_socket");
    result = bear_env_insert(result, "LD_PRELOAD", "/tmp/other_lib");

    char const * expected[] = 
        { "HOME=/home/user"
        , "LD_PRELOAD_NOW=what_is_this"
        , "BEAR_OUTPUT=/tmp/other_socket"
        , "LD_PRELOAD=/tmp/other_lib"
        , 0
        };
    assert_stringarray_equals(expected, result);

    sa_release(result);
}

void test_json() {
    char const * input_const[] =
        { "this"
        , "is my"
        , "message=\"shit\\gold\""
        , 0
        };
    Strings input = sa_copy(input_const);
    Strings result = bear_json_escape_strings(input);

    char const * expected[] = 
        { "this"
        , "\\\"is my\\\""
        , "message=\\\"shit\\\\gold\\\""
        , 0
        };
    assert_stringarray_equals(expected, result);

    sa_release(input);
}

void assert_messages_equals(struct bear_message const * lhs,
                            struct bear_message const * rhs) {
    assert(lhs->pid == rhs->pid);
    assert(0 == strcmp(lhs->fun, rhs->fun));
    assert(0 == strcmp(lhs->cwd, rhs->cwd));
    assert_stringarray_equals(lhs->cmd, rhs->cmd);
}

void test_protocol() {
    struct bear_message msg[2];
    {
        msg[1].pid = 9;
        msg[1].fun = "exec";
        msg[1].cwd = "/tmp";
        char const * cmds[] =
                { "this"
                , "that"
                , 0
                };
        msg[1].cmd = cmds;
    }
    {
        int fds[2];
        pipe(fds);

        bear_write_message(fds[1], &msg[1]);
        bear_read_message(fds[0], &msg[0]);
    }
    assert_messages_equals(&msg[1], &msg[0]);
}


int main() {
    test_sa_length();
    test_sa_fold();
    test_sa_append();
    test_sa_remove();
    test_sa_find();
    test_sa_copy();
    test_sa_build();
    test_env_insert();
    test_json();
    test_protocol();
    return 0;
}
