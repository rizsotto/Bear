// This file is distributed under MIT-LICENSE. See COPYING for details.

#include <unistd.h>
#include <malloc.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <sys/un.h>

// basic data structure is a buffer
struct Buffer {
    char * memory;
    unsigned int size;
    unsigned int current;
};

// destructor like method
static void buffer_free(struct Buffer * b) {
    free(b->memory);
    b->size = 0;
    b->current = 0;
}

// write the content into file
static void buffer_write(struct Buffer * b, int fd) {
    write(fd, b->memory, b->current);
}

// the only way to put anything into the buffer
static void buffer_put_char(struct Buffer * b, char c) {
    assert(b->size >= b->current);
    if (b->size <= b->current) {
        b->size += 4096;
        b->memory = (char *)realloc(b->memory, b->size);
    }
    b->memory[(b->current)++] = c;
    assert(b->size >= b->current);
}

// do write escape sequences if needed
static void buffer_put_escaped_char(struct Buffer * b, char c) {
    switch (c) {
    // it is not real json, only quotes are escaped
    case '\"' :
        buffer_put_char(b, '\\');
    default:
        buffer_put_char(b, c);
    }
}

static void buffer_put_word(struct Buffer * b, char const * const str) {
    char const * it = str;
    for (;*it;++it) {
        buffer_put_escaped_char(b, *it);
    }
}

static void buffer_put_many_words(struct Buffer * b, char const * const strs[]) {
    char const * const * it = strs;
    for (;*it;++it) {
        if (it != strs) {
            buffer_put_char(b, ' ');
        }
        buffer_put_word(b, *it);
    }
}

static void append_directory_entry(struct Buffer * b, char const * cwd) {
    buffer_put_char(b, '\"');
    buffer_put_word(b, "directory");
    buffer_put_char(b, '\"');
    buffer_put_word(b, " : ");
    buffer_put_char(b, '\"');
    buffer_put_word(b, cwd);
    buffer_put_char(b, '\"');
}

static void append_command_entry(struct Buffer * b, char const * const argv[]) {
    buffer_put_char(b, '\"');
    buffer_put_word(b, "command");
    buffer_put_char(b, '\"');
    buffer_put_word(b, " : ");
    buffer_put_char(b, '\"');
    buffer_put_many_words(b, argv);
    buffer_put_char(b, '\"');
}

static void write_call_info(int fd, char const * const argv[], char const *cwd) {
    struct Buffer b = { 0, 0, 0 };

    buffer_put_word(&b, "{ ");
    append_directory_entry(&b, cwd);
    buffer_put_word(&b, ", ");
    append_command_entry(&b, argv);
    buffer_put_word(&b, " }\n");

    buffer_write(&b, fd);

    buffer_free(&b);
}

void report_call(const char *method, char const * const argv[]) {
    // get current working dir
    static char buffer[4096];
    char const * const cwd = getcwd(buffer, sizeof(buffer));
    // get output file name
    char * const out = getenv("BEAR_OUTPUT");
    // call the real dumper
    if (out) {
        int s = socket(AF_UNIX, SOCK_STREAM, 0);
        if (-1 == s) {
            return;
        }
        struct sockaddr_un remote;
        remote.sun_family = AF_UNIX;
        strncpy(remote.sun_path, out, sizeof(remote.sun_path) - 1);
        if (-1 == connect(s, (struct sockaddr *)&remote, sizeof(struct sockaddr_un))) {
            close(s);
            return;
        }
        write_call_info(s, argv, cwd);
        close(s);
    }
}

