# - Finds C99 standard support
# This internally calls the check_c_source_compiles macro to determine the
# appropriate flags for a C99 standard compilation.

#=============================================================================
# Copyright 2013 Ian Liu Rodrigues <ian.liu88@gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
# 
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.
#=============================================================================

include(CheckCSourceCompiles)

set(C99_C_TEST_SOURCE
"#include <stdarg.h>
#include <stdbool.h>
#include <stdlib.h>
#include <wchar.h>
#include <stdio.h>

// Check varargs macros.  These examples are taken from C99 6.10.3.5.
#define debug(...) fprintf (stderr, __VA_ARGS__)
#define showlist(...) puts (#__VA_ARGS__)
#define report(test,...) ((test) ? puts (#test) : printf (__VA_ARGS__))
static void
test_varargs_macros (void)
{
  int x = 1234;
  int y = 5678;
  debug (\"Flag\");
  debug (\"X = %d\\\\n\", x);
  showlist (The first, second, and third items.);
  report (x>y, \"x is %d but y is %d\", x, y);
}

// Check long long types.
#define BIG64 18446744073709551615ull
#define BIG32 4294967295ul
#define BIG_OK (BIG64 / BIG32 == 4294967297ull && BIG64 % BIG32 == 0)
#if !BIG_OK
  your preprocessor is broken;
#endif
#if BIG_OK
#else
  your preprocessor is broken;
#endif
static long long int bignum = -9223372036854775807LL;
static unsigned long long int ubignum = BIG64;

struct incomplete_array
{
  int datasize;
  double data[];
};

struct named_init {
  int number;
  const wchar_t *name;
  double average;
};

typedef const char *ccp;

static inline int
test_restrict (ccp restrict text)
{
  // See if C++-style comments work.
  // Iterate through items via the restricted pointer.
  // Also check for declarations in for loops.
  for (unsigned int i = 0; *(text+i) != '\\\\0'; ++i)
    continue;
  return 0;
}

// Check varargs and va_copy.
static void
test_varargs (const char *format, ...)
{
  va_list args;
  va_start (args, format);
  va_list args_copy;
  va_copy (args_copy, args);

  const char *str = NULL;
  int number = 0;
  float fnumber = 0.0;

  while (*format)
    {
      switch (*format++)
	{
	case 's': // string
	  str = va_arg (args_copy, const char *);
	  break;
	case 'd': // int
	  number = va_arg (args_copy, int);
	  break;
	case 'f': // float
	  fnumber = va_arg (args_copy, double);
	  break;
	default:
	  break;
	}
    }
  va_end (args_copy);
  va_end (args);

  number = (number != 0) && (str != NULL) && (fnumber != 0.0);
}

int
main ()
{

  // Check bool.
  _Bool success = false;

  // Check restrict.
  if (test_restrict (\"String literal\") == 0)
    success = true;
  char *restrict newvar = \"Another string\";

  // Check varargs.
  test_varargs (\"s, d' f .\", \"string\", 65, 34.234);
  test_varargs_macros ();

  // Check flexible array members.
  struct incomplete_array *ia =
    malloc (sizeof (struct incomplete_array) + (sizeof (double) * 10));
  ia->datasize = 10;
  for (int i = 0; i < ia->datasize; ++i)
    ia->data[i] = i * 1.234;

  // Check named initializers.
  struct named_init ni = {
    .number = 34,
    .name = L\"Test wide string\",
    .average = 543.34343,
  };

  ni.number = 58;

  int dynamic_array[ni.number];
  dynamic_array[ni.number - 1] = 543;

  // work around unused variable warnings
  return (!success || bignum == 0LL || ubignum == 0uLL || newvar[0] == 'x'
	  || dynamic_array[ni.number - 1] != 543);

  ;
  return 0;
}")

set(C99_C_FLAG_CANDIDATES
   " "
   "-std=c99"
   "-std=gnu99"
   "-c99"
   "-AC99"
   "-xc99=all"
   "-qlanglvl=extc99"
)

if(DEFINED C99_C_FLAGS)
   set(C99_C_FLAG_CANDIDATES)
endif(DEFINED C99_C_FLAGS)

set(SAFE_CMAKE_REQUIRED_FLAGS "${CMAKE_REQUIRED_FLAGS}")
foreach(FLAG ${C99_C_FLAG_CANDIDATES})
   set(CMAKE_REQUIRED_FLAGS "${FLAG}")
   unset(C99_FLAG_DETECTED CACHE)
   message(STATUS "Try C99 C flag = [${FLAG}]")
   check_c_source_compiles("${C99_C_TEST_SOURCE}" C99_FLAG_DETECTED)
   if(C99_FLAG_DETECTED)
      set(C99_C_FLAGS_INTERNAL "${FLAG}")
      break()
   endif(C99_FLAG_DETECTED)
endforeach(FLAG ${C99_C_FLAG_CANDIDATES})
set(CMAKE_REQUIRED_FLAGS "${SAFE_CMAKE_REQUIRED_FLAGS}")

set(C99_C_FLAGS "${C99_C_FLAGS_INTERNAL}"
   CACHE STRING "C compiler flags for C99 standard")

mark_as_advanced(C99_C_FLAGS)