#include "boo.h++"
#include <foo.h>

void t2(int i);

void t1()
{
    for (int i = 0; i < 100; ++i)
    {
        if (98 == i)
        {
            t2(i);
            break;
        }
    }
}

void t2(int i)
{
    if (9 == i)
    {
        int k = i + 9;
        ++k;
        return;
    }
    acme::t1();
}

int main()
{
    t1();
    return 0;
}
