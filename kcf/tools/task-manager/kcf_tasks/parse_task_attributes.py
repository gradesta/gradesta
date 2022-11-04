from datetime import timedelta, datetime
from pytimeparse.timeparse import timeparse


class ParseError(Exception):
    pass


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
        try:
            return datetime.strptime(val.strip(), "%Y-%m-%d %H:%M")
        except ValueError:
            raise ParseError("Date could not be parsed " + val)
    return None


def get_money(line, tag):
    if val := get_tag_val(line, tag):
        toks = [t.strip() for t in val.split(" ")]
        if toks[0][0] != "$":
            raise ParseError("Expected dollar value as float preceded by $.")
        try:
            return float(toks[0][1:])
        except ValueError:
            raise ParseError("Expected dollar value as float preceded by $.")
    return None


def get_symbols(line, tag):
    """
    Get a value and return it as a list of words
    """
    if val := get_tag_val(line, tag):
        return [word.strip() for word in val.split(" ")]
    return None


class ParseError(Exception):
    pass


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
        try:
            return datetime.strptime(val.strip(), "%Y-%m-%d %H:%M")
        except ValueError:
            raise ParseError("Date could not be parsed " + val)
    return None


def get_money(line, tag):
    if val := get_tag_val(line, tag):
        toks = [t.strip() for t in val.split(" ")]
        if toks[0][0] != "$":
            raise ParseError("Expected dollar value as float preceded by $.")
        try:
            return float(toks[0][1:])
        except ValueError:
            raise ParseError("Expected dollar value as float preceded by $.")
    return None


def get_timedelta(line, tag):
    if val := get_tag_val(line, tag):
        if delta := timeparse(val) is not None:
            return delta
        else:
            raise ParseError(
                "Excpected duration in a format accepted by https://pypi.org/project/pytimeparse/"
            )
    return None


def get_symbols(line, tag):
    """
    Get a value and return it as a list of words
    """
    if val := get_tag_val(line, tag):
        return [word.strip() for word in val.split(" ")]
    return None


def parse_timedelta(val):
    if delta := timeparse(val) is not None:
        return delta
    else:
        raise ParseError(
            "Excpected duration in a format accepted by https://pypi.org/project/pytimeparse/"
        )


def get_time_log(line):
    if val := get_tag_val(line, "TASK_TIME_LOG"):
        symbols = [word.strip() for word in val.split(",")]
        if len(symbols) != 4:
            raise ParseError(
                "Expected the a comma separated line wiht date of the log entry, the task id, the author, and a duration in the format accepted by https://pypi.org/project/pytimeparse/. \
                Example: TASK_TIME_LOG: 2022-02-22, kjlsk28343847298, Timothy Hobbs, 3h"
            )
        date = datetime.strptime(symbols[0].strip(), "%Y-%m-%d")
        task_id = symbols[1]
        author = symbols[2]
        time_spent = parse_timedelta(symbols[3])
        return (date, task_id, author, time_spent)
    return None
