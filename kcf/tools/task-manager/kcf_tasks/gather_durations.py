import os
import yaml
import subprocess
from datetime import timedelta


def gather_durations(tasks, screencasts_folder=None):
    """
    Gather screencast durations from the `screencasts` directory that is found in the git repo and pair them with tasks.

    This function edits the tasks in place and doesn't return anything.
    """
    td = {}
    for task in tasks:
        td[task.TASK_ID] = task
    if screencasts_folder is None:
        try:
            screencasts_folder = (
                subprocess.check_output("git rev-parse --show-toplevel", shell=True)
                .decode("utf-8")
                .strip()
            ) + "/screencasts/"
        except subprocess.CalledProcessError:
            print("Not reading duration info from screencast metadata, not a git repo, no screencast metadata found.")
            return
    for metadata_file in next(os.walk(screencasts_folder))[2]:
        with open(screencasts_folder + metadata_file, "r") as fd:
            print("Reading metadata file", screencasts_folder, metadata_file)
            md = yaml.load(fd, Loader=yaml.Loader)
            if "tasks" not in md:
                continue
            duration = timedelta(seconds=md["duration_seconds"])
            task_weights = {}
            total_weight = 0
            for task_id in md["tasks"]:
                if task_id not in td:
                    continue
                task_weight = (
                    td[task_id].estimate_time_cost()["individual_work_max"].seconds
                )
                task_weights[task_id] = task_weight
                total_weight += task_weight
            for (task_id, weight) in task_weights.items():
                if weight == total_weight and total_weight == 0:
                    balanced_weight = 1
                else:
                    balanced_weight = weight / total_weight
                td[task_id].INVESTED_WORK_TIME += duration / balanced_weight
