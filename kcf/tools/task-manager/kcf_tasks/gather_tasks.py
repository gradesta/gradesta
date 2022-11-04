import subprocess
import sys
from datetime import timedelta, datetime
import os.path

from kcf_tasks.task import Task
from kcf_tasks.parse_task_attributes import ParseError, get_time_log


def gather_from_git(dir):
    """
    Returns a list of Task objects taken from the current git repo
    """
    output = subprocess.check_output(
        "git ls-files -- ':!kcf'", shell=True, cwd=dir
    ).decode("utf-8")

    tasks = []
    current_task = None
    for file in output.split("\n"):
        file = os.path.join(dir, file)
        if os.path.isfile(file):
            tasks += gather_from_file(file)
    return tasks


def gather_from_file(file):
    lineno = 0
    tasks = []
    task_time_logs = {}
    current_task = None
    with open(file) as fd:
        try:
            lines = fd.readlines()
        except UnicodeDecodeError:
            print("Could not decode file ", file, file=sys.stderr)
            return tasks
        for line in lines:
            lineno += 1
            try:
                if "TASK_TIME_LOG:" in line:
                    (date, task_id, author, time_spent) = get_time_log(line)
                    if task_id in task_time_logs:
                        task_time_logs[task_id].append((date, author, time_spent))
                    else:
                        task_time_logs[task_id] = [(date, author, time_spent)]
                if "TASK:" in line:
                    if current_task:
                        tasks.append(current_task)
                    current_task = Task()
                    current_task.SOURCE_FILE = file
                    current_task.START_LINE_IN_SOURCE_FILE = lineno
                if current_task:
                    current_task.read_line(line)
            except ParseError as e:
                sys.exit(
                    "[ERROR] {file}:{lineno} {line}\n{error}".format(
                        file=file, lineno=lineno, line=line, error=str(e)
                    )
                )
    if current_task:
        tasks.append(current_task)
    for task in tasks:
        if task.TASK_ID in task_time_logs:
            task.TASK_TIME_LOGs += task_time_logs[task.TASK_ID]
    return tasks
