// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef COMMON_STRINGARRAY_H
#define COMMON_STRINGARRAY_H

#include <stddef.h>

typedef char const *    String;
typedef String *        Strings;

Strings sa_copy(Strings const in);

Strings sa_append(Strings const in, String e);
void    sa_release(Strings);

size_t  sa_length(Strings const in);
int     sa_find(Strings const in, String e);

String  sa_fold(Strings const in, char const sep);

#endif
