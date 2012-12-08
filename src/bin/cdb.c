// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "cdb.h"
#include "../common/stringarray.h"

#include <unistd.h>
#include <string.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stddef.h>


static size_t count = 0;

// basic open/close methods with some decorator logic
int cdb_open(char const * file) {
    int fd = open(file, O_CREAT|O_RDWR, S_IRUSR|S_IWUSR);
    if (-1 == fd) {
        perror("open");
        exit(EXIT_FAILURE);
    }
    dprintf(fd, "[\n");
    count = 0;
    return fd;
}

void cdb_close(int fd) {
    dprintf(fd, "]\n");
    close(fd);
}


// data type and alloc/free methods
struct CDBEntry {
    char const * cwd;
    char const * cmd;
    char const * src;
};

struct CDBEntry * cdb_new() {
    struct CDBEntry * e = (struct CDBEntry *)malloc(sizeof(struct CDBEntry));
    e->src = 0;
    e->cmd = 0;
    e->cwd = 0;
    return e;
}

void cdb_delete(struct CDBEntry * e) {
    free((void *)e->src);
    free((void *)e->cmd);
    free((void *)e->cwd);
    free((void *)e);
}


// io related methods
static char const * read_string(int in);
static char const * get_source_file(char const * cmd, char const * cwd);

void cdb_read(int fd, struct CDBEntry * e) {
    e->cwd = read_string(fd);
    e->cmd = read_string(fd);
    e->src = get_source_file(e->cmd, e->cwd);
}

void cdb_write(int fd, struct CDBEntry const * e, int debug) {
    if (e->src) {
        if (count++) {
            dprintf(fd, ",\n");
        }
        dprintf(fd, "{\n"
                    "  \"directory\": \"%s\",\n"
                    "  \"command\": \"%s\",\n"
                    "  \"file\": \"%s\"\n"
                    "}\n", e->cwd, e->cmd, e->src);
    } else if (debug) {
        dprintf(fd, "#{\n"
                    "#  \"directory\": \"%s\",\n"
                    "#  \"command\": \"%s\"\n"
                    "#}\n", e->cwd, e->cmd);
    }
}


// length/value reader method, which hopefuly never blocks
static char const * read_string(int in) {
    size_t length = 0;
    if (-1 == read(in, (void *)&length, sizeof(size_t))) {
        perror("read: header");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        char * result = malloc(length + 1);
        if (-1 == read(in, (void *)result, length)) {
            perror("read: message");
            free(result);
            exit(EXIT_FAILURE);
        }
        result[length] = '\0';
        return result;
    }
    return "";
}

static int is_known_compiler(char const * cmd);
static int is_source_file(char const * const arg);

static char const * fix_path(char const * file, char const * cwd);


static char const * get_source_file(char const * cmd, char const * cwd) {
    Strings args = sa_unfold(cmd);
    char const * result = 0;
    // looking for compiler name
    if ((args) && (args[0]) && is_known_compiler(args[0])) {
        // looking for source file
        char const * const * it = args;
        for (; *it; ++it) {
            if (is_source_file(*it)) {
                result = fix_path(*it, cwd);
                break;
            }
        }
    }
    sa_release(args);
    return result;
}

static char const * fix_path(char const * file, char const * cwd) {
    if ('/' == file[0]) {
        return strdup(file);
    } else {
        size_t const sum_length = strlen(file) + strlen(cwd) + 2;
        char * result = (char *)malloc(sum_length);
        snprintf(result, sum_length, "%s/%s", cwd, file);
        return result;
    }
}

static int is_known_compiler(char const * cmd) {
    static char const * compilers[] =
        { "cc"
        , "gcc"
        , "llvm-gcc"
        , "clang"
        , "c++"
        , "g++"
        , "llvm-g++"
        , "clang++"
        , 0
        };

    // looking for compiler name
    char * file = basename(cmd);

    return sa_find(compilers, file);
}

static int is_source_file_extension(char const * arg);

static int is_source_file(char const * const arg) {
    char const * file_name = strrchr(arg, '/');
    file_name = (file_name) ? file_name : arg;
    char const * extension = strrchr(file_name, '.');
    extension = (extension) ? extension : file_name;

    return is_source_file_extension(extension);
}

static int is_source_file_extension(char const * arg) {
    static char const * extensions[] =
        { ".c"
        , ".C"
        , ".cc"
        , ".cxx"
        , ".c++"
        , ".C++"
        , ".cpp"
        , ".cp"
        , ".i"
        , ".ii"
        , ".m"
        , ".S"
        , 0
        };

    return sa_find(extensions, arg);
}

