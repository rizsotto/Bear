#!/usr/bin/env python
# -*- coding: utf-8 -*-

# Copyright (C) 2012-2019 by László Nagy
# This file is part of Bear.
#
# Bear is a tool to generate compilation database for clang tooling.
#
# Bear is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Bear is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
