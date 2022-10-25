from kcf_tasks.gather_tasks import gather_from_git, gather_from_file
from kcf_tasks.gather_durations import gather_durations

import os


def group_tasks_by_milestone(tasks):
    milestones = {"without-milestone": []}
    for task in tasks:
        for milestone in task.MILESTONES:
            if milestone not in milestones:
                milestones[milestone] = []
            milestones[milestone].append(task)
        if len(task.MANUAL_MILESTONES) == 0:
            milestones["without-milestone"].append(task)
    return milestones


def get_milestones(paths=None):
    """
    Gather tasks and group them by milestone. If a list of paths are passed, look only in those paths.
    """

    if paths is None:
        tasks = gather_from_git(os.getcwd())
    else:
        tasks = []
        for path in paths:
            if not os.path.isdir(path):
                tasks += gather_from_file(path)
            else:
                tasks += gather_from_git(path)

    gather_durations(tasks)

    return group_tasks_by_milestone(tasks)
