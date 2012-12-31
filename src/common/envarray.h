// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef COMMON_ENVARRAY_H
#define COMMON_ENVARRAY_H

#include "stringarray.h"

static char const * const ENV_PRELOAD = "LD_PRELOAD";
static char const * const ENV_OUTPUT  = "BEAR_OUTPUT";

Strings env_insert(Strings in, char const * key, char const * value);

#endif
