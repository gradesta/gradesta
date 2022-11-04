from dataclasses import dataclass, field
from datetime import datetime, timedelta
from kcf_tasks.time_cost_estimates import get_estimates
from kcf_tasks.parse_task_attributes import *

import sys


def initial_milestones():
    return ["all-tasks"]


@dataclass
class Task:
    NAME: str = ""
    TASK_ID: str = ""
    CREATED: datetime | None = None
    TIME_COST_ESTIMATES: [str] = field(default_factory=list)
    MILESTONES: [str] = field(default_factory=initial_milestones)
    INCOMPLETION_COST: float | None = None  # USD per hour
    START_VALUE: float | None = None  # USD
    MAX_VALUE: float | None = None  # USD
    BOUNTIED: datetime | None = None
    DESCRIPTION: str = ""
    SOURCE_FILE: str = ""
    START_LINE_IN_SOURCE_FILE: int = 0
    TASK_TIME_LOGs: [(datetime, timedelta, str)] = field(default_factory=list)

    def read_line(self, line):
        if "NO_TASK" in line:
            return
        if n := get_tag_val(line, "TASK"):
            self.NAME = n
        if i := get_tag_val(line, "TASK_ID"):
            self.TASK_ID = i
        if ms := get_symbols(line, "MILESTONES"):
            self.MILESTONES += ms
        if c := get_datetime(line, "CREATED"):
            self.CREATED = c
        if b := get_datetime(line, "BOUNTIED"):
            self.BOUNTIED = b
        if c := get_symbols(line, "ESTIMATED_TIME"):
            self.TIME_COST_ESTIMATES = c
        if ic := get_money(line, "INCOMPLETION_COST"):
            self.INCOMPLETION_COST = ic
        if sv := get_money(line, "START_VALUE"):
            self.START_VALUE = sv
        if mv := get_money(line, "MAX_VALUE"):
            self.MAX_VALUE = mv
        if d := get_tag_val(line, "DESCRIPTION"):
            self.DESCRIPTION = d

    def estimate_time_cost(self):
        return get_estimates(" ".join(self.TIME_COST_ESTIMATES or []))

    def summarize(self):
        return (
            str(self.estimate_time_cost()["individual_work_min"])
            + "-"
            + str(self.estimate_time_cost()["individual_work_max"])
            + ": "
            + self.NAME
        )

    @property
    def MANUAL_MILESTONES(self):
        return [m for m in self.MILESTONES if m != "all-tasks"]

    def __str__(self):
        s = "TASK: " + self.NAME + "\n"  # NO_TASK
        if self.TASK_ID:  # NO_TASK
            s += "TASK_ID: {}\n".format(self.TASK_ID)  # NO_TASK
        if self.CREATED:  # NO_TASK
            s += "CREATED: {}\n".format(  # NO_TASK
                self.CREATED.strftime("%Y-%m-%d %H:%M")
            )  # NO_TASK
        if self.TIME_COST_ESTIMATES:  # NO_TASK
            s += "ESTIMATED_TIME: {}\n".format(  # NO_TASK
                " ".join(self.TIME_COST_ESTIMATES)
            )  # NO_TASK
        if self.MILESTONES:  # NO_TASK
            s += "MILESTONES: {}\n".format(" ".join(self.MANUAL_MILESTONES))  # NO_TASK
        if self.INCOMPLETION_COST:  # NO_TASK
            s += "INCOMPLETION_COST: ${} per hour\n".format(  # NO_TASK
                self.INCOMPLETION_COST
            )  # NO_TASK
        if self.START_VALUE:  # NO_TASK
            s += "START_VALUE: ${}\n".format(self.START_VALUE)  # NO_TASK
        if self.MAX_VALUE:  # NO_TASK
            s += "MAX_VALUE: ${}\n".format(self.MAX_VALUE)  # NO_TASK
        if self.BOUNTIED:  # NO_TASK
            s += "BOUNTIED: {}\n".format(  # NO_TASK
                self.BOUNTIED.strftime("%Y-%m-%d %H:%M")
            )  # NO_TASK
        if self.DESCRIPTION:  # NO_TASK
            s += "DESCRIPTION: {}\n".format(self.DESCRIPTION)  # NO_TASK
        if self.TASK_TIME_LOGs:  # NO_TASK
            s += "TASK_TIME_LOGs: {}\n".format(self.TASK_TIME_LOGs)  # NO_TASK
        if self.SOURCE_FILE:  # NO_TASK
            s += "SOURCE: {}:{}\n".format(  # NO_TASK
                self.SOURCE_FILE, self.START_LINE_IN_SOURCE_FILE
            )
        return s


def test_read_line():
    task = Task()
    task.read_line("nothing interesting")
    assert task.TASK_ID == ""
    task.read_line("nothing interesting ** TASK: foo ")
    assert task.NAME == "foo"
    task.read_line("TASK_ID: abcd")  # NO_TASK
    assert task.TASK_ID == "abcd"
    task.read_line("TASK_ID: abcd  ")  # NO_TASK
    assert task.TASK_ID == "abcd"
    task.read_line("CREATED: 2022-08-12 10:20  ")  # NO_TASK
    assert task.CREATED.hour == 10
    task.read_line("ESTIMATED_TIME: U2 W4  ")  # NO_TASK
    assert ["U2", "W4"] == list(sorted(task.TIME_COST_ESTIMATES))
    task.read_line("MILESTONES: mvp abc  ")  # NO_TASK
    assert ["abc", "all-tasks", "mvp"] == list(sorted(task.MILESTONES))
    task.read_line("INCOMPLETION_COST: $3 per hour  ")  # NO_TASK
    assert 3.0 == task.INCOMPLETION_COST
    task.read_line("START_VALUE: $50 ")  # NO_TASK
    assert 50.0 == task.START_VALUE
    task.read_line("MAX_VALUE: $500 ")  # NO_TASK
    assert 500.0 == task.MAX_VALUE
    task.read_line("BOUNTIED:  2022-08-12 12:20  ")  # NO_TASK
    assert task.BOUNTIED.hour == 12
    task.read_line("DESCRIPTION:  Hello this is a big task ")  # NO_TASK
    assert task.DESCRIPTION == "Hello this is a big task"
    task.SOURCE_FILE = "foo.py"
    task.START_LINE_IN_SOURCE_FILE = 23
    assert task.estimate_time_cost()["individual_work_max"] == timedelta(hours=16)
    assert (
        str(task)
        == """TASK: foo
TASK_ID: abcd
CREATED: 2022-08-12 10:20
ESTIMATED_TIME: U2 W4
MILESTONES: mvp abc
INCOMPLETION_COST: $3.0 per hour
START_VALUE: $50.0
MAX_VALUE: $500.0
BOUNTIED: 2022-08-12 12:20
DESCRIPTION: Hello this is a big task
SOURCE: foo.py:23
"""
    )
