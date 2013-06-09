#!/bin/env python

import json
import sys

if __name__ == '__main__':
    print(len(json.load(sys.stdin)))
