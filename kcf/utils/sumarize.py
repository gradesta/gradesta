#!/usr/bin/python3
from gather_tasks import gather_from_git
from time_cost_estimates import get_empty_sums, add_sums
import sys

milestones = {"without-milestone": []}

for task in gather_from_git():
    if task.MILESTONES:
        for milestone in task.MILESTONES:
            if milestone not in milestones:
                milestones[milestone] = []
            milestones[milestone].append(task)
    else:
        milestones["without-milestone"].append(task)

for milestone, tasks in milestones.items():
    sums = get_empty_sums()
    for task in tasks:
        add_sums(sums, task.estimate_time_cost())

    print("\nMILESTONE: ", milestone)
    print("Minimum decision time:        ", sums["decision_min"])
    print("Maximum decision time:        ", sums["decision_max"])

    print(
        "Minimum individual work time: ",
        sums["individual_work_min"].total_seconds() / 3600,
        " hours",
    )
    print(
        "Maximum individual work time: ",
        sums["individual_work_max"].total_seconds() / 3600,
        " hours",
    )


    print(
        "Minimum team work time:       ",
        sums["team_work_min"].total_seconds() / 3600,
        " hours",
    )
    print(
        "Maximum team work time:       ",
        sums["team_work_max"].total_seconds() / 3600,
        " hours",
    )
