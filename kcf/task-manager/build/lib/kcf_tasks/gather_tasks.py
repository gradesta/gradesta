import subprocess
import sys
from datetime import timedelta, datetime
from kcf_tasks.task import Task
import os.path

def gather_from_git():
    """
    Returns a list of Task objects taken from the current git repo
    """
    output = subprocess.check_output("git ls-files -- ':!kcf'", shell=True).decode("utf-8")

    tasks = []
    current_task = None
    for file in output.split("\n"):
        if os.path.isfile(file):
            tasks += gather_from_file(file)
    return tasks

def gather_from_file(file):
    lineno = 0
    tasks = []
    current_task = None
    with open(file) as fd:
        try:
            lines = fd.readlines()
        except UnicodeDecodeError:
            print("Could not decode file ", file, file=sys.stderr)
            return tasks
        for line in lines:
            lineno += 1
            if "TASK: " in line:
                if current_task:
                    tasks.append(current_task)
                current_task = Task()
                current_task.SOURCE_FILE = file
                current_task.START_LINE_IN_SOURCE_FILE = lineno
            if current_task:
                current_task.read_line(line)
    return tasks
