// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BEAR_ENVIRON_H
#define BEAR_ENVIRON_H

static char const * const ENV_PRELOAD = "LD_PRELOAD";
static char const * const ENV_OUTPUT  = "BEAR_OUTPUT";

char const * * bear_env_insert(char const * * in, char const * key, char const * value);
char * * bear_get_environ(void);

#endif
