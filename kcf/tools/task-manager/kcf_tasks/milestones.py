from datetime import timedelta
import os

from kcf_tasks.gather_tasks import gather_from_git, gather_from_file
from kcf_tasks.time_cost_estimates import get_empty_sums, add_sums
from kcf_tasks.gather_durations import gather_durations


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


def get_milestones(paths=None, milestone="/"):
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
    if not milestone[-1] == "/":

        def check_milestone(task):
            return milestone in task.MILESTONES

    else:

        def check_milestone(task):
            for tm in task.MILESTONES:
                if tm.startswith(milestone[:-1]):
                    return True
            return False

    tasks = [task for task in tasks if check_milestone(task)]

    gather_durations(tasks)

    return group_tasks_by_milestone(tasks)


def sum_estimates(tasks):
    sums = get_empty_sums()
    for task in tasks:
        add_sums(sums, task.estimate_time_cost())
    return sums


def sum_time_spend(tasks):
    total_time_spent = timedelta(seconds=0)
    for task in tasks:
        total_time_spent += task.time_spent()
    return total_time_spent


def print_milestone(milestone, tasks):
    sums = sum_estimates(tasks)
    total_time_spent = sum_time_spend(tasks)
    print("\nMILESTONE: ", milestone)
    print("Minimum decision time:          ", sums["decision_min"])
    print("Maximum decision time:          ", sums["decision_max"])
    print(
        "Minimum team work time:         ",
        sums["team_work_min"].total_seconds() / 3600,
        " hours",
    )
    print(
        "Maximum team work time:         ",
        sums["team_work_max"].total_seconds() / 3600,
        " hours",
    )

    print(
        "Minimum individual work time:   ",
        sums["individual_work_min"].total_seconds() / 3600,
        " hours",
    )
    print(
        "Maximum individual work time:   ",
        sums["individual_work_max"].total_seconds() / 3600,
        " hours",
    )
    print(
        "Original estimate for complete: ",
        sums["individual_work_estimated_completed_min"].total_seconds() / 3600,
        " → ",
        sums["individual_work_estimated_completed_max"].total_seconds() / 3600,
        " hours",
    )
    print("Total time spent:               ", total_time_spent)
    print("Completed tasks:                ", sums["completed"])
    print("Incomplete tasks:               ", sums["incomplete"])


def print_milestones(milestones):
    for milestone, tasks in milestones.items():
        print_milestone(milestone, tasks)
