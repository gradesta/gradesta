import os
import yaml
import subprocess

def gather_durations(tasks, screencasts_folder=None):
    """
    Gather screencast durations from the `screencasts` directory that is found in the git repo and pair them with tasks.
    """
    td = {}
    for task in tasks:
        td[task.TASK_ID] = task
    if screencasts_folder is None:
        screencasts_folder = (
            subprocess.check_output("git rev-parse --show-toplevel", shell=True)
            .decode("utf-8")
            .strip()
        ) + "/screencasts/"
    for metadata_file in next(os.walk(screencasts_folder))[2]:
        with open(screencasts_folder + metadata_file, "r") as fd:
            print("Reading metadata file", screencasts_folder,  metadata_file)
            md = yaml.load(fd, Loader=yaml.Loader)
