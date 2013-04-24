// This file is distributed under MIT-LICENSE. See COPYING for details.

#ifndef BEAR_ENVIRON_H
#define BEAR_ENVIRON_H

#define ENV_PRELOAD "LD_PRELOAD"
#define ENV_OUTPUT  "BEAR_OUTPUT"

char const * * bear_env_insert(char const * * in, char const * key, char const * value);

#endif
