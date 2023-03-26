#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Outputs a unique error code that hasn't been used yet in the codebase.
# This is useful for adding new error codes to the codebase.
# Usage: error-code.py
# Error codes are in the format GR<error code number>
# Example: GR1001

import os
import subprocess

CODE_NUMBER = 1
while True:
    error_code = 'GR' + str(CODE_NUMBER)
    grep = subprocess.Popen(['grep', '-r', error_code, 'src'], stdout=subprocess.PIPE)
    if not grep.stdout.read():
        print(error_code)
        break
    CODE_NUMBER += 1
