/*  Copyright (C) 2012, 2013 by László Nagy
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

#include "environ.h"
#include "stringarray.h"

#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef NEED_NSGETENVIRON 
#include <crt_externs.h>
#else
// Some platforms don't provide environ in any header.
extern char **environ;
#endif


char const * * bear_update_environ(char const * envs[], char const * key)
{
    char const * const value = getenv(key);
    if (0 == value)
    {
        perror("bear: getenv");
        exit(EXIT_FAILURE);
    }
    // find the key if it's there
    size_t const key_length = strlen(key);
    char const * * it = envs;
    for (; (it) && (*it); ++it)
    {
        if ((0 == strncmp(*it, key, key_length)) &&
            (strlen(*it) > key_length) &&
            ('=' == (*it)[key_length]))
            break;
    }
    // check the value might already correct
    char const * * result = envs;
    if ((0 != it) && ((0 == *it) || (strcmp(*it + key_length + 1, value))))
    {
        char * env = 0;
        if (-1 == asprintf(&env, "%s=%s", key, value))
        {
            perror("bear: asprintf");
            exit(EXIT_FAILURE);
        }
        if (*it)
        {
            free((void *)*it);
            *it = env;
        }
        else
            result = bear_strings_append(envs, env);
    }
    return result;
}

char * * bear_get_environ(void)
{
#ifdef NEED_NSGETENVIRON 
    // environ is not available for shared libraries have to use _NSGetEnviron()
    return *_NSGetEnviron();
#else
    return environ;
#endif
}
