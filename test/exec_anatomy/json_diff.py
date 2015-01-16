#!/usr/bin/env python

import sys
import json

def main():
    try:
        lhs = {pretty(entry) for entry in load(sys.argv[1])}
        rhs = {pretty(entry) for entry in load(sys.argv[2])}
        if len(lhs - rhs):
            for e in lhs - rhs:
                print('> {0}'.format(e))
            for e in rhs - lhs:
                print('< {0}'.format(e))
            return 40
        return 0
    except Exception as ex:
        print(ex)
        return 50


def load(filename):
    with open(filename, 'r') as handler:
        return json.load(handler)


def pretty(entry):
    return str(sorted(entry.items(), key=lambda x: x[0]))


if __name__ == '__main__':
    sys.exit(main())
