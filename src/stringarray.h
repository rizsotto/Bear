// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BEAR_STRINGS_H
#define BEAR_STRINGS_H

#include <stddef.h>

#ifdef CLIENT
#include <stdarg.h>
char const ** bear_strings_copy(char const ** const in);
char const ** bear_strings_build(char const * arg, va_list ap);
#endif

char const ** bear_strings_append(char const ** in, char const * e);
char const ** bear_strings_remove(char const ** in, char const * e);

size_t        bear_strings_length(char const * const * in);
int           bear_strings_find(char const * const * in, char const * e);

void          bear_strings_release(char const **);

#ifdef SERVER
char const *  bear_strings_fold(char const * const * in, char const sep);
#endif

#endif
