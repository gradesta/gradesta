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


def get_milestones(paths=None, milestone="/", completion_filter="ALL"):
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

    def task_filter(task):
        if check_milestone(task):
            if completion_filter == "ALL":
                return True
            if completion_filter == "INCOMPLETE":
                return not "DONE" in task.TIME_COST_ESTIMATES
            if completion_filter == "COMPLETE":
                return "DONE" in task.TIME_COST_ESTIMATES
            sys.exit("Invalid completion filter %s" % completion_filter)
        return False

    # Link tasks into task tree
    tasks_by_task_id = {task.TASK_ID: task for task in tasks}
    for task in tasks:
        if task.PARENT:
            task.parent_ptr = tasks_by_task_id[task.PARENT]
            task.parent_ptr.subtask_ptrs.append(task)

    tasks_ = tasks
    tasks = []
    # We topologically sort tasks so that we can do milestone inheritance from parent tasks to child tasks.
    # See task 5bf3f2c74ac49bff9016e98b4eb42391
    milestones_by_task_id = {}
    while tasks_:
        task = tasks_[-1]
        parent = task.PARENT
        if parent == "" or parent in milestones_by_task_id:
            if parent in milestones_by_task_id:
                task.MILESTONES = task.MILESTONES.union(milestones_by_task_id[parent])
            milestones_by_task_id[task.TASK_ID] = task.MILESTONES
            tasks.append(tasks_.pop())
        else:
            tasks_ = [tasks_.pop()] + tasks_

    tasks = [task for task in tasks if task_filter(task)]

    gather_durations(tasks)

    return group_tasks_by_milestone(tasks)


def sum_estimates(tasks):
    sums = get_empty_sums()
    incomplete = 0
    complete = 0
    for task in tasks:
        if not task.PARENT:
            add_sums(sums, task.estimate_time_cost())
        if task.is_done():
            complete += 1
        else:
            incomplete += 1
    return sums, complete, incomplete


def sum_time_spend(tasks):
    total_time_spent = timedelta(seconds=0)
    for task in tasks:
        if not task.PARENT:
            total_time_spent += task.time_spent()
    return total_time_spent


def print_milestone(milestone, tasks):
    sums, complete, incomplete = sum_estimates(tasks)
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
    print("Completed tasks:                ", complete)
    print("Incomplete tasks:               ", incomplete)


def print_milestones(milestones):
    for milestone, tasks in milestones.items():
        print_milestone(milestone, tasks)
