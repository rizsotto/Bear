#!/bin/env python

from os import listdir
from os.path import isdir, join
import subprocess
import logging
import re

def _test(bear, path):
    config = '{}/config'.format(path)
    cmd = [bear, '-c', config, '--', 'true']
    logging.debug('executing {}'.format(cmd))
    child = subprocess.Popen(cmd, stderr=subprocess.PIPE)
    child.wait()
    return child.stderr.read()

def _read(path):
    file = '{}/output'.format(path)
    logging.debug('reading {}'.format(file))
    with open(file, "r") as fd:
        return fd.read()

def _evaulate(bear, path):
    result = _test(bear, path)
    regex = re.compile(_read(path).encode())
    if (None == regex.match(result)):
        logging.error('failed test: {}'.format(path))
        logging.info('stderr: {}'.format(result))
        return False
    return True

def main():
    from argparse import ArgumentParser
    parser = ArgumentParser()
    parser.add_argument("--sut",
                        metavar='FILE',
                        required=True,
                        help="SUT executable")
    parser.add_argument("--test-cases",
                        metavar='DIRECTORY',
                        required=True,
                        help="where the tests files are")
    parser.add_argument('--log-level',
                        metavar='LEVEL',
                        choices='DEBUG INFO WARN ERROR'.split(),
                        default='INFO',
                        help="Choose a level from DEBUG, INFO (default), \
                              WARN or ERROR")
    args = parser.parse_args()

    logging.basicConfig(format='%(levelname)s: %(message)s', level=args.log_level)

    bear = args.sut
    parent = args.test_cases
    subdirs = [ join(parent,d) for d in listdir(parent) if isdir(join(parent,d)) ]
    results = [ _evaulate(bear, t) for t in subdirs ]
    return all(results)

if __name__ == '__main__':
    import sys
    if (main()):
        sys.exit(0)
    else:
        sys.exit(1)
