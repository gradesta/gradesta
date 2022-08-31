#!/usr/bin/python3
import subprocess
from datetime import timedelta
import sys

output = subprocess.check_output("git grep -h TIME_COST -- ':!kcf'", shell=True).decode("utf-8")
output = "\n".join([l for l in output.splitlines() if "DONE" not in l])
costs = {
    "U0": ("decision", timedelta(minutes=0), timedelta(minutes=15)),
    "U1": ("decision", timedelta(minutes=15), timedelta(hours=4)),
    "U2": ("decision", timedelta(hours=2), timedelta(days=2)),
    "U3": ("decision", timedelta(days=1), timedelta(weeks=1)),
    "U4": ("decision", timedelta(days=3), timedelta(weeks=2)),
    "U5": ("decision", timedelta(weeks=1), timedelta(weeks=4)),
    "U6": ("decision", timedelta(weeks=2), timedelta(weeks=12)),
    "U7": ("decision", timedelta(weeks=8), timedelta(weeks=24)),
    "U8": ("decision", timedelta(weeks=16), timedelta(weeks=52)),
    "W1": ("individual_work", timedelta(minutes=5), timedelta(minutes=45)),
    "W2": ("individual_work", timedelta(minutes=15), timedelta(hours=1)),
    "W3": ("individual_work", timedelta(minutes=30), timedelta(hours=4)),
    "W4": ("individual_work", timedelta(hours=1), timedelta(hours=16)),
    "W5": ("individual_work", timedelta(hours=4), timedelta(hours=32)),
    "W6": ("individual_work", timedelta(hours=8), timedelta(hours=64)),
    "W7": ("individual_work", timedelta(hours=16), timedelta(hours=128)),
    "W8": ("individual_work", timedelta(hours=32), timedelta(hours=256)),
    "T1": ("team_work", timedelta(hours=10), timedelta(hours=20)),
    "T2": ("team_work", timedelta(hours=15), timedelta(hours=30)),
    "T3": ("team_work", timedelta(hours=20), timedelta(hours=40)),
    "T4": ("team_work", timedelta(hours=30), timedelta(hours=60)),
    "T5": ("team_work", timedelta(hours=40), timedelta(hours=80)),
    "T6": ("team_work", timedelta(hours=60), timedelta(hours=120)),
    "T7": ("team_work", timedelta(hours=80), timedelta(hours=160)),
    "T8": ("team_work", timedelta(hours=100), timedelta(hours=200)),
    "T9": ("team_work", timedelta(hours=150), timedelta(hours=300)),
    "T10": ("team_work", timedelta(hours=200), timedelta(hours=400)),
    "T11": ("team_work", timedelta(hours=300), timedelta(hours=800)),
    "T12": ("team_work", timedelta(hours=600), timedelta(hours=1200)),
}

sums = {
    "decision_min": timedelta(seconds=0),
    "decision_max": timedelta(seconds=0),
    "individual_work_min": timedelta(seconds=0),
    "individual_work_max": timedelta(seconds=0),
    "team_work_min": timedelta(seconds=0),
    "team_work_max": timedelta(seconds=0),
}

for classification, (type, min, max) in costs.items():
    sums[type + "_min"] += output.count(classification) * min
    sums[type + "_max"] += output.count(classification) * max

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
