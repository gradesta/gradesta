from kcf_tasks.milestones import get_milestones

def list_tasks_(source_dir=None):
    milestones = get_milestones()

    for task in milestones["all-tasks"]:
        print(task.summarize())
