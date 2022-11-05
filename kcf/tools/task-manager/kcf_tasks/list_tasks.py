from kcf_tasks.milestones import get_milestones


def list_tasks_(milestones):
    for task in milestones["all-tasks"]:
        print(task.summarize())
