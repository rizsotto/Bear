#!/usr/bin/env python

import argparse
import json
import sys
import os.path


def diff(lhs, rhs):
    left = {smooth(entry): entry for entry in lhs}
    right = {smooth(entry): entry for entry in rhs}
    for key in left.keys():
        if key not in right:
            yield '> {}'.format(left[key])
    for key in right.keys():
        if key not in left:
            yield '< {}'.format(right[key])


def smooth(entry):
    directory = os.path.normpath(entry['directory'])
    source = entry['file'] if os.path.isabs(entry['file']) else \
        os.path.normpath(os.path.join(directory, entry['file']))
    arguments = entry['command'].split() if 'command' in entry else \
        entry['arguments']
    return '-'.join([source[::-1]] + arguments)


def main():
    """ Semantically diff two compilation databases. """
    parser = argparse.ArgumentParser()
    parser.add_argument('left', type=argparse.FileType('r'))
    parser.add_argument('right', type=argparse.FileType('r'))
    args = parser.parse_args()
    # files are open, parse the json content
    lhs = json.load(args.left)
    rhs = json.load(args.right)
    # run the diff and print the result
    count = 0
    for result in diff(lhs, rhs):
        print(result)
        count += 1
    return count


if __name__ == '__main__':
    sys.exit(main())
