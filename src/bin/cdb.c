// This file is distributed under MIT-LICENSE. See COPYING for details.

#include "cdb.h"

#include <unistd.h>
#include <string.h>
#include <malloc.h>
#include <stdlib.h>
#include <stdio.h>
#include <sys/stat.h>
#include <fcntl.h>


int cdb_open(char const * file) {
    int ofd = open(file, O_CREAT|O_RDWR, S_IRUSR|S_IWUSR);
    if (-1 == ofd) {
        perror("open");
        exit(EXIT_FAILURE);
    }
    return ofd;
}

void cdb_close(int ofd) {
    close(ofd);
}


static char const * read_string(int in);
static char const * get_source_file(char const * cmd);

void cdb_copy(int ofd, int ifd) {
    char const * const cwd = read_string(ifd);
    char const * const cmd = read_string(ifd);
    char const * const file = get_source_file(cmd);
    if (file) {
        write(ofd, cwd, strlen(cwd));
        write(ofd, "\n", 1);
        write(ofd, cmd, strlen(cmd));
        write(ofd, "\n", 1);
        write(ofd, file, strlen(file));
        write(ofd, "\n", 1);
    }
    free((void *)file);
    free((void *)cmd);
    free((void *)cwd);
}


static char const * read_string(int in) {
    size_t length = 0;
    if (-1 == read(in, (void *)&length, sizeof(size_t))) {
        perror("read: header");
        exit(EXIT_FAILURE);
    }
    if (length > 0) {
        char * result = malloc(length + 1);
        if (-1 == read(in, (void *)result, length)) {
            free(result);
            perror("read: message");
            exit(EXIT_FAILURE);
        }
        result[length] = '\0';
        return result;
    }
    return "";
}

static char const * const * create_tokens(char const * in);
static void release_tokens(char const * const * mem);

static int is_known_compiler(char const * cmd);
static int is_source_file(char const * const arg);

static char const * get_source_file(char const * cmd) {
    char const * const * args = create_tokens(cmd);
    char const * result = 0;
    // looking for compiler name
    if ((args) && (args[0]) && is_known_compiler(args[0])) {
        // looking for source file
        char const * const * it = args;
        for (; *it; ++it) {
            if (is_source_file(*it)) {
                result = strdup(*it);
                break;
            }
        }
    }
    release_tokens(args);
    return result;
}

static char const * const * create_tokens(char const * in) {
    char const * * result = 0;
    size_t result_size = 0;
    if (in) {
        char * const copy = strdup(in);
        if (0 == copy) {
            perror("strdup");
            exit(EXIT_FAILURE);
        }
        // find separators and insert 0 in place
        char * it = copy;
        do {
            result = (char const * *)realloc(result, (result_size + 1) * sizeof(char const *));
            if (0 == result) {
                perror("realloc");
                exit(EXIT_FAILURE);
            }
            result[result_size++] = it;
            char * sep = strchr(it, ' ');
            if (sep) {
                *sep = '\0';
                ++sep;
            }
            it = sep;
        } while (it);
        result = (char const * *)realloc(result, (result_size + 1) * sizeof(char const *));
        if (0 == result) {
            perror("realloc");
            exit(EXIT_FAILURE);
        }
        result[result_size++] = 0;
    }
    return result;
}

static void release_tokens(char const * const * mem) {
    if (mem) {
        if (*mem) {
            free((void *)(*mem));
        }
        free((void *)mem);
    }
}

static int is_known_compiler(char const * cmd) {
    static char const * const compilers[] =
        { "cc"
        , "gcc"
        , "llvm-gcc"
        , "clang"
        , "c++"
        , "g++"
        , "llvm-g++"
        , "clang++"
        };
    static size_t const compilers_size =
        sizeof(compilers) / sizeof(char const * const);

    int result = 0;
    // looking for compiler name
    char * file = basename(cmd);
    if (file) {
        size_t idx = 0;
        for (;compilers_size > idx; ++idx) {
            if (0 == strcmp(file, compilers[idx])) {
                ++result;
                break;
            }
        }
    }
    return result;
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
    static char const * const extensions[] =
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
        };
    static size_t const extensions_size =
        sizeof(extensions) / sizeof(char const * const);

    size_t idx = 0;
    for (;extensions_size > idx; ++idx) {
        if (0 == strcmp(arg, extensions[idx])) {
            return 1;
        }
    }
    return 0;
}

