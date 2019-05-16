/*  Copyright (C) 2012-2019 by László Nagy
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

/**
 * This file implements a shared library. This library can be pre-loaded by
 * the dynamic linker of the Operating System (OS). It implements a few function
 * related to process creation. By pre-load this library the executed process
 * uses these functions instead of those from the standard library.
 *
 * The idea here is to inject a logic before call the real methods. The logic is
 * to dump the call into a file. To call the real method this library is doing
 * the job of the dynamic linker.
 *
 * The only input for the log writing is about the destination directory.
 * This is passed as environment variable.
 */

#include "config.h"

#include <stddef.h>
#include <stdarg.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <locale.h>
#include <unistd.h>
#include <dlfcn.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <fcntl.h>
#include <pthread.h>
#include <errno.h>
#include <limits.h>

#if defined HAVE_POSIX_SPAWN || defined HAVE_POSIX_SPAWNP
#include <spawn.h>
#endif

#if defined HAVE_NSGETENVIRON
# include <crt_externs.h>
static char **environ;
#else
extern char **environ;
#endif

#define ENV_OUTPUT "INTERCEPT_BUILD_TARGET_DIR"
#ifdef APPLE
# define ENV_FLAT    "DYLD_FORCE_FLAT_NAMESPACE"
# define ENV_PRELOAD "DYLD_INSERT_LIBRARIES"
# define ENV_SIZE 3
#else
# define ENV_PRELOAD "LD_PRELOAD"
# define ENV_SIZE 2
#endif

#define STRINGIFY(x) #x
#define TOSTRING(x) STRINGIFY(x)
#define AT "libear: (" __FILE__ ":" TOSTRING(__LINE__) ") "

#define PERROR(msg) do { perror(AT msg); } while (0)

#define ERROR_AND_EXIT(msg) do { PERROR(msg); exit(EXIT_FAILURE); } while (0)

#define DLSYM(TYPE_, VAR_, SYMBOL_)                                 \
    union {                                                         \
        void *from;                                                 \
        TYPE_ to;                                                   \
    } cast;                                                         \
    if (0 == (cast.from = dlsym(RTLD_NEXT, SYMBOL_))) {             \
        PERROR("dlsym");                                            \
        exit(EXIT_FAILURE);                                         \
    }                                                               \
    TYPE_ const VAR_ = cast.to;


typedef char const * bear_env_t[ENV_SIZE];

#define FIFO_HEADER_SIZE 32
const int max_fifo_payload_size = (PIPE_BUF - FIFO_HEADER_SIZE);

enum Report_Type {
    REPORT_FIFO,
    REPORT_TEMP_FILE
};

struct Report_buf {
    enum Report_Type report_type_;
    char *report_file_name_;
    int temp_file_fd_;
    char *report_buf_;
    int report_capacity_;
    int current_pos_;
    int initial_report_size_;
    int delete_temp_file_when_done_;
};

static int capture_env_t(bear_env_t *env);
static void release_env_t(bear_env_t *env);
static char const **string_array_partial_update(char *const envp[], bear_env_t *env);
static char const **string_array_single_update(char const *envs[], char const *key, char const *value);
static void report_call(char const *const argv[]);
static int write_report(struct Report_buf *report_buf, char const *const argv[]);
static char const **string_array_from_varargs(char const * arg, va_list *args);
static char const **string_array_copy(char const **in);
static size_t string_array_length(char const *const *in);
static void string_array_release(char const **);
static void report_buf_construct(struct Report_buf *report_buf);
static void report_buf_destroy(struct Report_buf *report_buf);
static int report_buf_write(struct Report_buf *report_buf, const void *buf, size_t count);
static void report_buf_write_fifo(struct Report_buf *report_buf);

static bear_env_t env_names =
    { ENV_OUTPUT
    , ENV_PRELOAD
#ifdef ENV_FLAT
    , ENV_FLAT
#endif
    };

static bear_env_t initial_env =
    { 0
    , 0
#ifdef ENV_FLAT
    , 0
#endif
    };

static int initialized = 0;
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;

static void on_load(void) __attribute__((constructor));
static void on_unload(void) __attribute__((destructor));

static int mt_safe_on_load(void);
static void mt_safe_on_unload(void);


#ifdef HAVE_EXECVE
static int call_execve(const char *path, char *const argv[],
                       char *const envp[]);
#endif
#ifdef HAVE_EXECVP
static int call_execvp(const char *file, char *const argv[]);
#endif
#ifdef HAVE_EXECVPE
static int call_execvpe(const char *file, char *const argv[],
                        char *const envp[]);
#endif
#ifdef HAVE_EXECVP2
static int call_execvP(const char *file, const char *search_path,
                       char *const argv[]);
#endif
#ifdef HAVE_EXECT
static int call_exect(const char *path, char *const argv[],
                      char *const envp[]);
#endif
#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid, const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict]);
#endif
#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid, const char *restrict file,
                             const posix_spawn_file_actions_t *file_actions,
                             const posix_spawnattr_t *restrict attrp,
                             char *const argv[restrict],
                             char *const envp[restrict]);
#endif


/* Initialization method to Captures the relevant environment variables.
 */

static void on_load(void) {
    pthread_mutex_lock(&mutex);
    if (0 == initialized)
        initialized = mt_safe_on_load();
    pthread_mutex_unlock(&mutex);
}

static void on_unload(void) {
    pthread_mutex_lock(&mutex);
    if (0 != initialized)
        mt_safe_on_unload();
    initialized = 0;
    pthread_mutex_unlock(&mutex);
}

static int mt_safe_on_load(void) {
#ifdef HAVE_NSGETENVIRON
    environ = *_NSGetEnviron();
    if (0 == environ)
        return 0;
#endif
    // Capture current relevant environment variables
    return capture_env_t(&initial_env);
}

static void mt_safe_on_unload(void) {
    release_env_t(&initial_env);
}


/* These are the methods we are try to hijack.
 */

#ifdef HAVE_EXECVE
int execve(const char *path, char *const argv[], char *const envp[]) {
    report_call((char const *const *)argv);
    return call_execve(path, argv, envp);
}
#endif

#ifdef HAVE_EXECV
#ifndef HAVE_EXECVE
#error can not implement execv without execve
#endif
int execv(const char *path, char *const argv[]) {
    report_call((char const *const *)argv);
    return call_execve(path, argv, environ);
}
#endif

#ifdef HAVE_EXECVPE
int execvpe(const char *file, char *const argv[], char *const envp[]) {
    report_call((char const *const *)argv);
    return call_execvpe(file, argv, envp);
}
#endif

#ifdef HAVE_EXECVP
int execvp(const char *file, char *const argv[]) {
    report_call((char const *const *)argv);
    return call_execvp(file, argv);
}
#endif

#ifdef HAVE_EXECVP2
int execvP(const char *file, const char *search_path, char *const argv[]) {
    report_call((char const *const *)argv);
    return call_execvP(file, search_path, argv);
}
#endif

#ifdef HAVE_EXECT
int exect(const char *path, char *const argv[], char *const envp[]) {
    report_call((char const *const *)argv);
    return call_exect(path, argv, envp);
}
#endif

#ifdef HAVE_EXECL
# ifndef HAVE_EXECVE
#  error can not implement execl without execve
# endif
int execl(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = string_array_from_varargs(arg, &args);
    va_end(args);

    report_call((char const *const *)argv);
    int const result = call_execve(path, (char *const *)argv, environ);

    string_array_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLP
# ifndef HAVE_EXECVP
#  error can not implement execlp without execvp
# endif
int execlp(const char *file, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = string_array_from_varargs(arg, &args);
    va_end(args);

    report_call((char const *const *)argv);
    int const result = call_execvp(file, (char *const *)argv);

    string_array_release(argv);
    return result;
}
#endif

#ifdef HAVE_EXECLE
# ifndef HAVE_EXECVE
#  error can not implement execle without execve
# endif
// int execle(const char *path, const char *arg, ..., char * const envp[]);
int execle(const char *path, const char *arg, ...) {
    va_list args;
    va_start(args, arg);
    char const **argv = string_array_from_varargs(arg, &args);
    char const **envp = va_arg(args, char const **);
    va_end(args);

    report_call((char const *const *)argv);
    int const result =
        call_execve(path, (char *const *)argv, (char *const *)envp);

    string_array_release(argv);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWN
int posix_spawn(pid_t *restrict pid, const char *restrict path,
                const posix_spawn_file_actions_t *file_actions,
                const posix_spawnattr_t *restrict attrp,
                char *const argv[restrict], char *const envp[restrict]) {
    report_call((char const *const *)argv);
    return call_posix_spawn(pid, path, file_actions, attrp, argv, envp);
}
#endif

#ifdef HAVE_POSIX_SPAWNP
int posix_spawnp(pid_t *restrict pid, const char *restrict file,
                 const posix_spawn_file_actions_t *file_actions,
                 const posix_spawnattr_t *restrict attrp,
                 char *const argv[restrict], char *const envp[restrict]) {
    report_call((char const *const *)argv);
    return call_posix_spawnp(pid, file, file_actions, attrp, argv, envp);
}
#endif

/* These are the methods which forward the call to the standard implementation.
 */

#ifdef HAVE_EXECVE
static int call_execve(const char *path, char *const argv[],
                       char *const envp[]) {
    typedef int (*func)(const char *, char *const *, char *const *);

    DLSYM(func, fp, "execve");

    char const **const menvp = string_array_partial_update(envp, &initial_env);
    int const result = (*fp)(path, argv, (char *const *)menvp);
    string_array_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVPE
static int call_execvpe(const char *file, char *const argv[],
                        char *const envp[]) {
    typedef int (*func)(const char *, char *const *, char *const *);

    DLSYM(func, fp, "execvpe");

    char const **const menvp = string_array_partial_update(envp, &initial_env);
    int const result = (*fp)(file, argv, (char *const *)menvp);
    string_array_release(menvp);
    return result;
}
#endif

#ifdef HAVE_EXECVP
static int call_execvp(const char *file, char *const argv[]) {
    typedef int (*func)(const char *file, char *const argv[]);

    DLSYM(func, fp, "execvp");

    char **const original = environ;
    char const **const modified = string_array_partial_update(original, &initial_env);
    environ = (char **)modified;
    int const result = (*fp)(file, argv);
    environ = original;
    string_array_release(modified);

    return result;
}
#endif

#ifdef HAVE_EXECVP2
static int call_execvP(const char *file, const char *search_path,
                       char *const argv[]) {
    typedef int (*func)(const char *, const char *, char *const *);

    DLSYM(func, fp, "execvP");

    char **const original = environ;
    char const **const modified = string_array_partial_update(original, &initial_env);
    environ = (char **)modified;
    int const result = (*fp)(file, search_path, argv);
    environ = original;
    string_array_release(modified);

    return result;
}
#endif

#ifdef HAVE_EXECT
static int call_exect(const char *path, char *const argv[],
                      char *const envp[]) {
    typedef int (*func)(const char *, char *const *, char *const *);

    DLSYM(func, fp, "exect");

    char const **const menvp = string_array_partial_update(envp, &initial_env);
    int const result = (*fp)(path, argv, (char *const *)menvp);
    string_array_release(menvp);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWN
static int call_posix_spawn(pid_t *restrict pid, const char *restrict path,
                            const posix_spawn_file_actions_t *file_actions,
                            const posix_spawnattr_t *restrict attrp,
                            char *const argv[restrict],
                            char *const envp[restrict]) {
    typedef int (*func)(pid_t *restrict, const char *restrict,
                        const posix_spawn_file_actions_t *,
                        const posix_spawnattr_t *restrict,
                        char *const *restrict, char *const *restrict);

    DLSYM(func, fp, "posix_spawn");

    char const **const menvp = string_array_partial_update(envp, &initial_env);
    int const result =
        (*fp)(pid, path, file_actions, attrp, argv, (char *const *restrict)menvp);
    string_array_release(menvp);
    return result;
}
#endif

#ifdef HAVE_POSIX_SPAWNP
static int call_posix_spawnp(pid_t *restrict pid, const char *restrict file,
                             const posix_spawn_file_actions_t *file_actions,
                             const posix_spawnattr_t *restrict attrp,
                             char *const argv[restrict],
                             char *const envp[restrict]) {
    typedef int (*func)(pid_t *restrict, const char *restrict,
                        const posix_spawn_file_actions_t *,
                        const posix_spawnattr_t *restrict,
                        char *const *restrict, char *const *restrict);

    DLSYM(func, fp, "posix_spawnp");

    char const **const menvp = string_array_partial_update(envp, &initial_env);
    int const result =
        (*fp)(pid, file, file_actions, attrp, argv, (char *const *restrict)menvp);
    string_array_release(menvp);
    return result;
}
#endif

static void report_buf_construct(struct Report_buf *report_buf) {
    report_buf->delete_temp_file_when_done_ = 0;
    char const *const out_dir = initial_env[0];
    size_t const path_max_length = strlen(out_dir) + 32;
    report_buf->report_file_name_ = malloc(path_max_length);
    if (0 == report_buf->report_file_name_) {
        ERROR_AND_EXIT("malloc");
    }

    // First check if fifo file exists, if not will move on to writing to temp file.
    if (-1 == snprintf(report_buf->report_file_name_, path_max_length, "%s/bearfifo", out_dir)) {
        ERROR_AND_EXIT("snprintf");
    }

    struct stat stat_buffer;
    if (stat(report_buf->report_file_name_, &stat_buffer) == 0) {
        // Fifo file exists, report into the FIFO
        report_buf->report_type_ = REPORT_FIFO;
        report_buf->temp_file_fd_ = -1;

        report_buf->report_capacity_ = max_fifo_payload_size;
        report_buf->report_buf_ = malloc(report_buf->report_capacity_);
        if (0 == report_buf) ERROR_AND_EXIT("malloc");
        report_buf->current_pos_ = 0;
    } else {
        // No fifo, Report data into a temp file
        report_buf->report_type_ = REPORT_TEMP_FILE;

        if (-1 == snprintf(report_buf->report_file_name_, path_max_length, "%s/execution.XXXXXX", out_dir)) {
            ERROR_AND_EXIT("snprintf");
        }
        // Create report file
        report_buf->temp_file_fd_ = mkstemp(report_buf->report_file_name_);
        if (-1 == report_buf->temp_file_fd_) {
            ERROR_AND_EXIT("mkstemp");
        }
        report_buf->report_buf_ = NULL;
    }
}

static void report_buf_destroy(struct Report_buf *report_buf) {
    if (report_buf->temp_file_fd_ != -1) {
        if (close(report_buf->temp_file_fd_)) {
            ERROR_AND_EXIT("close");
        }
        report_buf->temp_file_fd_ = -1;
    }

    if ((1 == report_buf->delete_temp_file_when_done_) && (-1 == unlink(report_buf->report_file_name_))) {
        ERROR_AND_EXIT("unlink");
    }

    if (report_buf->report_buf_ != NULL) {
        free((void *)(report_buf->report_buf_));
        report_buf->report_buf_ = NULL;
    }
    if (report_buf->report_file_name_ != NULL) {
        free((void *)(report_buf->report_file_name_));
        report_buf->report_file_name_ = NULL;
    }
}

static int report_buf_write(struct Report_buf *report_buf, const void *buf, size_t count) {
    if (REPORT_TEMP_FILE == report_buf->report_type_) {
        int write_ret = write(report_buf->temp_file_fd_, buf, count);
        if (-1 == write_ret) {
            // Caller responsible for meaningful error message
            // Mark delete_temp_file_when_done_ so temp file gets deleted later.
            report_buf->delete_temp_file_when_done_ = 1;
        }
        return write_ret;
    }

    // Code from here is for FIFO only. REPORT_TEMP_FILE code never reaches here.
    while (1) {
        char *ptr_to_write_to = &(report_buf->report_buf_[report_buf->current_pos_]);

        int amt_remaining_in_report = report_buf->report_capacity_ - report_buf->current_pos_;
        if (count <= amt_remaining_in_report) {
            memcpy(ptr_to_write_to, buf, count);
            report_buf->current_pos_ += count;
            return count;
        }

        // Not enough space, increase the capacity and then try again.
        int amt_more_needed = report_buf->current_pos_ + count - report_buf->report_capacity_;

        // ceil (amt_more_needed/report_buf->report_capacity_) to see how much to add
        int num_max_payloads_to_add = (amt_more_needed + max_fifo_payload_size - 1) / max_fifo_payload_size;
        report_buf->report_capacity_ = report_buf->report_capacity_ + num_max_payloads_to_add * max_fifo_payload_size;
        report_buf->report_buf_ = realloc(report_buf->report_buf_, report_buf->report_capacity_);
        if (0 == report_buf->report_buf_) {
            ERROR_AND_EXIT("realloc");
        }
    }
}

static void report_buf_write_fifo(struct Report_buf *report_buf) {
    int fd_fifo = open(report_buf->report_file_name_, O_WRONLY);
    if (-1 == fd_fifo) {
        ERROR_AND_EXIT("fifo open");
    }

    // According to pipe documentation at: http://man7.org/linux/man-pages/man7/pipe.7.html
    // "POSIX.1 says that write(2)s of less than PIPE_BUF bytes must be atomic"
    // Since there is a possibility that the report will be larger than PIPE_BUF, we need
    // to break the report into packets of max size PIPE_BUF each.
    char *fifo_packet = malloc(PIPE_BUF);
    if (0 == fifo_packet) {
        ERROR_AND_EXIT("malloc");
    }

    pid_t pid = getpid();

    int got_error = 0;

    // Calculate the total # of required parts
    // ceiling ( report_size / max_fifo_payload_size )
    int total_parts = (report_buf->current_pos_ + (max_fifo_payload_size - 1)) / max_fifo_payload_size;
    int part_idx;
    for (part_idx = 0; part_idx < total_parts; ++part_idx) {
        int packet_pos_in_report_buf = part_idx * max_fifo_payload_size;
        int payload_size = max_fifo_payload_size;
        if (part_idx == (total_parts - 1)) {
            payload_size = report_buf->current_pos_ - packet_pos_in_report_buf;
        }

        // Header layout, fit exactly into 32 bytes
        // 12345678 12345 12345 12345678  1
        // 01234567890123456789012345678901
        // pay_size part# totpa pidxxxxx  N
        // The formatting in snprintf below is designed to exactly fit in 32 bytes
        // as the FIFO header. Add 1 to make room for null character byte which will
        // get overwritten with the payload.
        snprintf(fifo_packet, FIFO_HEADER_SIZE + 1, "%8d %5d %5d %8d  \n", payload_size, part_idx, total_parts, pid);

        // Copy the payload of the packet into the FIFO packet buffer after the header.
        memcpy(&(fifo_packet[FIFO_HEADER_SIZE]), &(report_buf->report_buf_[packet_pos_in_report_buf]), payload_size);

        // Write the FIFO packet into the FIFO buffer
        if (-1 == write(fd_fifo, fifo_packet, FIFO_HEADER_SIZE + payload_size)) {
            PERROR("write fifo");
            got_error = 1;
            break;
        }
    }

    free((void *)fifo_packet);
    close(fd_fifo);
    if (got_error) {
        ERROR_AND_EXIT("FIFO writing error!");
    }
}

/* this method is to write log about the process creation. */

static void report_call(char const *const argv[]) {
    if (!initialized)
        return;

    // Create report buffer
    struct Report_buf report_buf;
    report_buf_construct(&report_buf);
    // Write report to the buffer
    write_report(&report_buf, argv);

    if (REPORT_FIFO == report_buf.report_type_)
    {
      // Write report to the fifo
      report_buf_write_fifo(&report_buf);
    }
    // Destroy report buffer
    report_buf_destroy(&report_buf);
}

static int write_binary_string(struct Report_buf *report_buf, const char *const string) {
    // write type
    if (-1 == report_buf_write(report_buf, "str", 3)) {
        PERROR("write type");
        return -1;
    }
    // write length
    const uint32_t length = strlen(string);
    if (-1 == report_buf_write(report_buf, (void *) &length, sizeof(uint32_t))) {
        PERROR("write length");
        return -1;
    }
    // write value
    if (-1 == report_buf_write(report_buf, (void *) string, length)) {
        PERROR("write value");
        return -1;
    }
    return 0;
}

static int write_binary_string_list(struct Report_buf *report_buf, const char *const *const strings) {
    // write type
    if (-1 == report_buf_write(report_buf, "lst", 3)) {
        PERROR("write type");
        return -1;
    }
    // write length
    const uint32_t length = string_array_length(strings);
    if (-1 == report_buf_write(report_buf, (void *) &length, sizeof(uint32_t))) {
        PERROR("write length");
        return -1;
    }
    // write value
    for (uint32_t idx = 0; idx < length; ++idx) {
        const char *string = strings[idx];
        if (-1 == write_binary_string(report_buf, string)) {
            PERROR("write value");
            return -1;
        }
    }
    return 0;
}

static int write_report(struct Report_buf *report_buf, char const *const argv[]) {
    const char *cwd = getcwd(NULL, 0);
    if (0 == cwd) {
        PERROR("getcwd");
        return -1;
    } else {
        if (-1 == write_binary_string(report_buf, cwd)) {
            PERROR("cwd writing failed");
            return -1;
        }
    }
    free((void *)cwd);
    if (-1 == write_binary_string_list(report_buf, argv)) {
        PERROR("cmd writing failed");
        return -1;
    }
    return 0;
}

/* update environment assure that chilren processes will copy the desired
 * behaviour */

static int capture_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it) {
        char const * const env_value = getenv(env_names[it]);
        if (0 == env_value) {
            PERROR("getenv");
            return 0;
        }

        char const * const env_copy = strdup(env_value);
        if (0 == env_copy) {
            PERROR("strdup");
            return 0;
        }

        (*env)[it] = env_copy;
    }
    return 1;
}

static void release_env_t(bear_env_t *env) {
    for (size_t it = 0; it < ENV_SIZE; ++it) {
        free((void *)(*env)[it]);
        (*env)[it] = 0;
    }
}

static char const **string_array_partial_update(char *const envp[], bear_env_t *env) {
    char const **result = string_array_copy((char const **)envp);
    for (size_t it = 0; it < ENV_SIZE && (*env)[it]; ++it)
        result = string_array_single_update(result, env_names[it], (*env)[it]);
    return result;
}

static char const **string_array_single_update(char const *envs[], char const *key, char const * const value) {
    // find the key if it's there
    size_t const key_length = strlen(key);
    char const **it = envs;
    for (; (it) && (*it); ++it) {
        if ((0 == strncmp(*it, key, key_length)) &&
            (strlen(*it) > key_length) && ('=' == (*it)[key_length]))
            break;
    }
    // allocate a environment entry
    size_t const value_length = strlen(value);
    size_t const env_length = key_length + value_length + 2;
    char *env = malloc(env_length);
    if (0 == env)
        ERROR_AND_EXIT("malloc");
    if (-1 == snprintf(env, env_length, "%s=%s", key, value))
        ERROR_AND_EXIT("snprintf");
    // replace or append the environment entry
    if (it && *it) {
        free((void *)*it);
        *it = env;
	    return envs;
    } else {
        size_t const size = string_array_length(envs);
        char const **result = realloc(envs, (size + 2) * sizeof(char const *));
        if (0 == result)
            ERROR_AND_EXIT("realloc");
        result[size] = env;
        result[size + 1] = 0;
        return result;
    }
}

/* util methods to deal with string arrays. environment and process arguments
 * are both represented as string arrays. */

static char const **string_array_from_varargs(char const *const arg, va_list *args) {
    char const **result = 0;
    size_t size = 0;
    for (char const *it = arg; it; it = va_arg(*args, char const *)) {
        result = realloc(result, (size + 1) * sizeof(char const *));
        if (0 == result)
            ERROR_AND_EXIT("realloc");
        char const *copy = strdup(it);
        if (0 == copy)
            ERROR_AND_EXIT("strdup");
        result[size++] = copy;
    }
    result = realloc(result, (size + 1) * sizeof(char const *));
    if (0 == result)
        ERROR_AND_EXIT("realloc");
    result[size++] = 0;

    return result;
}

static char const **string_array_copy(char const **const in) {
    size_t const size = string_array_length(in);

    char const **const result = malloc((size + 1) * sizeof(char const *));
    if (0 == result)
        ERROR_AND_EXIT("malloc");

    char const **out_it = result;
    for (char const *const *in_it = in; (in_it) && (*in_it);
         ++in_it, ++out_it) {
        *out_it = strdup(*in_it);
        if (0 == *out_it)
            ERROR_AND_EXIT("strdup");
    }
    *out_it = 0;
    return result;
}

static size_t string_array_length(char const *const *const in) {
    size_t result = 0;
    for (char const *const *it = in; (it) && (*it); ++it)
        ++result;
    return result;
}

static void string_array_release(char const **in) {
    for (char const *const *it = in; (it) && (*it); ++it) {
        free((void *)*it);
    }
    free((void *)in);
}
