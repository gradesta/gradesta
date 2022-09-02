from dataclasses import dataclass, field
from datetime import datetime, timedelta
from kcf_tasks.time_cost_estimates import get_estimates


def get_tag_val(line, tag):
    """
    Returns tag value as unparsed string or None if tag is not present
    """
    tag = tag + ":"
    if tag in line:
        return line.split(tag)[1].strip()
    return None


def get_datetime(line, tag):
    """
    Returns a datetime if tag is present, otherwise returns None
    """
    if val := get_tag_val(line, tag):
        return datetime.strptime(val.strip(), "%Y-%m-%d %H:%M")
    return None


def get_money(line, tag):
    if val := get_tag_val(line, tag):
        toks = [t.strip() for t in val.split(" ")]
        assert toks[0][0] == "$"
        return float(toks[0][1:])
    return None


def get_symbols(line, tag):
    """
    Get a value and return it as a list of words
    """
    if val := get_tag_val(line, tag):
        return [word.strip() for word in val.split(" ")]
    return None


@dataclass
class Task:
    NAME: str = ""
    TASK_ID: str = ""
    CREATED: datetime | None = None
    TIME_COST_ESTIMATES: [str] = field(default_factory=list)
    MILESTONES: [str] = field(default_factory=list)
    INCOMPLETION_COST: float | None = None  # USD per hour
    START_VALUE: float | None = None  # USD
    MAX_VALUE: float | None = None  # USD
    BOUNTIED: datetime | None = None
    DESCRIPTION: str = ""
    SOURCE_FILE: str = ""
    START_LINE_IN_SOURCE_FILE: int = 0

    def read_line(self, line):
        if "NO_TASK" in line:
            return
        if n := get_tag_val(line, "TASK"):
            self.NAME = n
        if i := get_tag_val(line, "TASK_ID"):
            self.TASK_ID = i
        if ms := get_symbols(line, "MILESTONES"):
            self.MILESTONES = ms
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

    def __str__(self):
        s = "TASK: " + self.NAME + "\n"
        if self.TASK_ID:
            s += "TASK_ID: {}\n".format(self.TASK_ID)
        if self.CREATED:
            s += "CREATED: {}\n".format(self.CREATED.strftime("%Y-%m-%d %H:%M"))
        if self.TIME_COST_ESTIMATES:
            s += "ESTIMATED_TIME: {}\n".format(" ".join(self.TIME_COST_ESTIMATES))
        if self.MILESTONES:
            s += "MILESTONES: {}\n".format(" ".join(self.MILESTONES))
        if self.INCOMPLETION_COST:
            s += "INCOMPLETION_COST: ${} per hour\n".format(self.INCOMPLETION_COST)
        if self.START_VALUE:
            s += "START_VALUE: ${}\n".format(self.START_VALUE)
        if self.MAX_VALUE:
            s += "MAX_VALUE: ${}\n".format(self.MAX_VALUE)
        if self.BOUNTIED:
            s += "BOUNTIED: {}\n".format(self.BOUNTIED.strftime("%Y-%m-%d %H:%M"))
        if self.DESCRIPTION:
            s += "DESCRIPTION: {}\n".format(self.DESCRIPTION)
        if self.SOURCE_FILE:
            s += "SOURCE: {}:{}\n".format(
                self.SOURCE_FILE, self.START_LINE_IN_SOURCE_FILE
            )
        return s


def test_read_line():
    task = Task()
    task.read_line("nothing interesting")
    assert task.TASK_ID == ""
    task.read_line("nothing interesting ** TASK: foo ")
    assert task.NAME == "foo"
    task.read_line("TASK_ID: abcd")
    assert task.TASK_ID == "abcd"
    task.read_line("TASK_ID: abcd  ")
    assert task.TASK_ID == "abcd"
    task.read_line("CREATED: 2022-08-12 10:20  ")
    assert task.CREATED.hour == 10
    task.read_line("ESTIMATED_TIME: U2 W4  ")
    assert ["U2", "W4"] == list(sorted(task.TIME_COST_ESTIMATES))
    task.read_line("MILESTONES: mvp abc  ")
    assert ["abc", "mvp"] == list(sorted(task.MILESTONES))
    task.read_line("INCOMPLETION_COST: $3 per hour  ")
    assert 3.0 == task.INCOMPLETION_COST
    task.read_line("START_VALUE: $50 ")
    assert 50.0 == task.START_VALUE
    task.read_line("MAX_VALUE: $500 ")
    assert 500.0 == task.MAX_VALUE
    task.read_line("BOUNTIED:  2022-08-12 12:20  ")
    assert task.BOUNTIED.hour == 12
    task.read_line("DESCRIPTION:  Hello this is a big task ")
    assert task.DESCRIPTION == "Hello this is a big task"
    task.SOURCE_FILE = "foo.py"
    task.START_LINE_IN_SOURCE_FILE = 23
    assert task.estimate_time_cost()["individual_work_max"] == timedelta(hours=16)
    assert str(task) == """TASK: foo
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
