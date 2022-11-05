import json as json_

from kcf_tasks.milestones import get_milestones


def list_tasks_(milestones, json=False):
    if json:
        tasks = milestones["all-tasks"]
        task_representations = []
        for task in tasks:
            task_representations.append(task.json_like_dict())
        print(json_.dumps(task_representations))
    else:
        for task in milestones["all-tasks"]:
            print(task.summarize())
