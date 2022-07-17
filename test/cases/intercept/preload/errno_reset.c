// REQUIRES: preload
// RUN: %{compile} -o %t %s
// RUN: %{intercept} --verbose --output %t.json -- %t

#include <stdlib.h>
#include <errno.h>
#include <string.h>

int main()
{
    printf("error %i: %s\n", errno, strerror(errno));
    return 0;
}
