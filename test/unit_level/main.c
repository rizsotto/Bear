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
#include <protocol.h>

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

void test_io(int fd) {
    char const * const in_msg_1 = "this is\x02my\x1fmessage!";
    char const * const in_msg_2 = "";

    write_string(fd, in_msg_1);
    write_string(fd, in_msg_2);
    write_string(fd, 0);

    assert(0 == lseek(fd, 0, SEEK_SET));

    char const * const out_msg_1 = read_string(fd);
    char const * const out_msg_2 = read_string(fd);
    char const * const out_msg_3 = read_string(fd);

    assert(0 == strcmp(in_msg_1, out_msg_1));
    assert(0 == strcmp(in_msg_2, out_msg_2));
    assert(0 == strcmp(in_msg_2, out_msg_3));

    free((void *)out_msg_1);
    free((void *)out_msg_2);
    free((void *)out_msg_3);
}

void test_protocol() {
    char const * const file_name = "protocol_test";
    int fd = shm_open(file_name, O_CREAT|O_RDWR, S_IRUSR|S_IWUSR);
    assert(-1 != fd);
    test_io(fd);
    close(fd);
    shm_unlink(file_name);
}

int main() {
    test_sa_length();
    test_sa_fold();
    test_sa_unfold();
    test_sa_unfold_fold();
    test_sa_append();
    test_sa_find();
    test_sa_copy();
    test_protocol();
    return 0;
}
