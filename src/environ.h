// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BEAR_ENVIRON_H
#define BEAR_ENVIRON_H

#include "config.h"

#ifdef APPLE
#define ENV_PRELOAD "DYLD_INSERT_LIBRARIES"
#define ENV_FLAT "DYLD_FORCE_FLAT_NAMESPACE"
#else
#define ENV_PRELOAD "LD_PRELOAD"
#endif
#define ENV_OUTPUT  "BEAR_OUTPUT"

char const * * bear_env_insert(char const * * in, char const * key, char const * value);
char * * bear_get_environ(void);

#endif
