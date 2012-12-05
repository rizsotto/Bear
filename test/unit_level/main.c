// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <assert.h>

#include <malloc.h>
#include <string.h>

#include <stringarray.h>

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
    char const * const expected = "this is my message";
    char const * const result = sa_fold(input);

    assert((0 == strcmp(expected, result)) && "sa_fold failed");

    free((void *)result);
}

void test_sa_unfold() {
    char const * const input = " this  is my   message  ";
    Strings const result = sa_unfold(input);

    assert(4 == sa_length(result));
    assert(0 == strcmp("this",      result[0]));
    assert(0 == strcmp("is",        result[1]));
    assert(0 == strcmp("my",        result[2]));
    assert(0 == strcmp("message",   result[3]));

    sa_release(result);
}

void test_sa_unfold_fold() {
    char const * const input = "this is my message";
    Strings const middle = sa_unfold(input);
    char const * const result = sa_fold(middle);

    assert(0 == strcmp(input, result));

    sa_release(middle);
    free((void *)result);
}

void test_sa_append() {
    Strings result = 0;

    result = sa_append(result, "this");

    assert(1 == sa_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == result[1]);

    result = sa_append(result, "that");

    assert(2 == sa_length(result));
    assert(0 == strcmp("this", result[0]));
    assert(0 == strcmp("that", result[1]));
    assert(0 == result[2]);
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
}

int main() {
    test_sa_length();
    test_sa_fold();
    test_sa_unfold();
    test_sa_unfold_fold();
    test_sa_append();
    test_sa_find();
    test_sa_copy();
    return 0;
}
