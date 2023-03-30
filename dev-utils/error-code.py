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
    error_code = "GR" + str(CODE_NUMBER)
    # We have to duplicate the exclude-dir patterns as the subprocess call doesn't do this, only the shell does.
    # grep -r GR --exclude-dir={.git,.svn,CVS} --exclude={*.png,*.jpg,*.gif,*.zip,*.gz,*.bz2,*.tar,*.tgz,*.rar,*.exe,*.dll,*.obj,*.o,*.a,*.lib,*.so,*.class,*.psd} .
    grep = subprocess.Popen(
        [
            "grep",
            "-r",
            error_code,
            "--exclude-dir=.git",
            "--exclude-dir=target",
            "--exclude-dir=node_modules",
            "--exclude-dir=dist",
            "--exclude-dir=venv",
            "--exclude-dir=3rd-party",
            "--exclude=*.png",
            "--exclude=*.jpg",
            "--exclude=*.gif",
            "--exclude=*.zip",
            "--exclude=*.gz",
            "--exclude=*.bz2",
            "--exclude=*.tar",
            "--exclude=*.tgz",
            "--exclude=*.rar",
            "--exclude=*.exe",
            "--exclude=*.dll",
            "--exclude=*.obj",
            "--exclude=*.o",
            "--exclude=*.a",
            "--exclude=*.lib",
            "--exclude=*.so",
            "--exclude=*.class",
            "--exclude=*.psd",
            "--exclude=*.bin",
            "--exclude=*.xoj",
            "--exclude=package-lock.json",
            ".",
        ],
        stdout=subprocess.PIPE,
    )
    out = grep.stdout.read()
    print(out)
    if not out:
        print(error_code)
        break

    CODE_NUMBER += 1
