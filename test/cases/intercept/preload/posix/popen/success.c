// REQUIRES: preload, c_api_popen
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.sqlite3 -- %t
// RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
// RUN: assert_intercepted %t.json count -ge 2
// RUN: assert_intercepted %t.json contains -program %t
// RUN: assert_intercepted %t.json contains -arguments more

#include <stdio.h>
#include <stdlib.h>

void
write_data (FILE * stream)
{
  int i;
  for (i = 0; i < 100; i++)
    fprintf (stream, "%d\n", i);
  if (ferror (stream))
    {
      fprintf (stderr, "Output to stream failed.\n");
      exit (EXIT_FAILURE);
    }
}

int
main (void)
{
  FILE *output;

  output = popen ("more", "w");
  if (!output)
    {
      fprintf (stderr,
               "incorrect parameters or too many files.\n");
      return EXIT_FAILURE;
    }
  write_data (output);
  if (pclose (output) != 0)
    {
      fprintf (stderr,
               "Could not run more or other error.\n");
    }
  return EXIT_SUCCESS;
}
