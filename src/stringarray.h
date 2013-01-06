// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef COMMON_STRINGARRAY_H
#define COMMON_STRINGARRAY_H

#include <stddef.h>
#include <stdarg.h>

#ifdef CLIENT
char const ** sa_copy(char const ** const in);
char const ** sa_build(char const * arg, va_list ap);
#endif

char const ** sa_append(char const ** in, char const * e);
char const ** sa_remove(char const ** in, char const * e);

size_t  sa_length(char const * const * in);
int     sa_find(char const * const * in, char const * e);

void    sa_release(char const **);

#ifdef SERVER
char const * sa_fold(char const * const * in, char const sep);
#endif

#endif
