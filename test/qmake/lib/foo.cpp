#include "foo.h"

namespace acme {

void t2(int i);

void t1() {
    for (int i = 0; i < 100; ++i) {
        if (98 == i) {
            t2(i);
            break;
        }
    }
}

void t2(int i) {
    if (9 == i) {
        int k = i + 9;
        --k;
        return;
    }
}

}
