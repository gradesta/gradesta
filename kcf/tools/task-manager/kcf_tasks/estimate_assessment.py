from dataclasses import dataclass, field
from datetime import timedelta
from collections import OrderedDict
from math import ceil

from kcf_tasks.milestones import get_milestones
from kcf_tasks.time_cost_estimates import costs


@dataclass
class EstimateType:
    label: str
    total_time_spend: timedelta = field(default_factory=timedelta)
    num_low: int = 0
    num_high: int = 0
    n: int = 0
    estimate_min: timedelta = field(default_factory=timedelta)
    estimate_max: timedelta = field(default_factory=timedelta)

    def estimate_str(self):
        return "{} - {}".format(str(self.estimate_min), str(self.estimate_max)).ljust(
            20
        )

    def actual_average_str(self):
        avg = self.total_time_spend / self.n
        avg = avg - timedelta(microseconds=avg.microseconds) # round to seconds
        return str(avg).rjust(14)

    def accuracy_str(self):
        return "{}%".format(
            ceil(100 * (self.n - self.num_low - self.num_high) / self.n)
        ).ljust(8)

    def err_low_str(self):
        return "{}%".format(ceil(100 * self.num_low / self.n)).rjust(4)

    def err_high_str(self):
        return "{}%".format(ceil(100 * self.num_high / self.n)).rjust(6)

    def __str__(self):
        return "{} {} {} {} {}{}{}".format(
            self.label,
            self.estimate_str(),
            self.actual_average_str(),
            str(self.n).rjust(4),
            self.accuracy_str(),
            self.err_low_str(),
            self.err_high_str(),
        )


def assess_estimates(source_dir=None):
    milestones = get_milestones()

    estimate_types = OrderedDict()
    for label, cost in costs.items():
        if label.startswith("W"):
            estimate_types[label] = EstimateType(
                label=label, estimate_min=cost[1], estimate_max=cost[2]
            )

    for task in milestones["all-tasks"]:
        if "DONE" in task.TIME_COST_ESTIMATES:
            for label, et in estimate_types.items():
                if label in task.TIME_COST_ESTIMATES:
                    et.n += 1
                    time_spent = task.time_spent()
                    if time_spent > et.estimate_max:
                        et.num_high += 1
                    elif time_spent < et.estimate_min:
                        et.num_low += 1
                    et.total_time_spend += time_spent
    return estimate_types


def assess_estimates_(source):
    ets = assess_estimates(source)
    print("       estimate         actual average    n accuracy low  high")
    for et in ets.values():
        if et.n > 0:
            print(et)
