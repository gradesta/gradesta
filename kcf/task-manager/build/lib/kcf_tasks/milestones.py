from kcf_tasks.gather_tasks import gather_from_git, gather_from_file


def group_tasks_by_milestone(tasks):
    milestones = {"without-milestone": []}
    for task in tasks:
        if task.MILESTONES:
            for milestone in task.MILESTONES:
                if milestone not in milestones:
                    milestones[milestone] = []
                milestones[milestone].append(task)
        else:
            milestones["without-milestone"].append(task)
    return milestones

def get_milestones(paths = None):
    """
    Gather tasks and group them by milestone. If a list of paths are passed, look only in those paths.
    """

    if paths is None:
        tasks = gather_from_git()
    else:
        tasks = []
        for path in paths:
            if isfile(path):
                tasks += gather_from_file(path)
            else:
                tasks += gather_from_git(folder)

    return group_tasks_by_milestone(tasks)
