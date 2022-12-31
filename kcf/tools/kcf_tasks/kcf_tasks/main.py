#!/usr/bin/python3
import sys
import os
from argparse import ArgumentParser

from kcf_tasks.milestones import print_milestones, get_milestones
from kcf_tasks.list_tasks import list_tasks_
from kcf_tasks.estimate_assessment import assess_estimates_


def get_milestones_(opts):
    completion_filter = "ALL"
    if opts.complete:
        completion_filter = "COMPLETE"
    if opts.incomplete:
        completion_filter = "INCOMPLETE"
    return get_milestones(
        opts.source, milestone=opts.milestone, completion_filter=completion_filter
    )


def list_tasks(opts):
    list_tasks_(get_milestones_(opts), json=opts.json)


def list_milestones(opts):
    print_milestones(get_milestones_(opts))


def assess_estimates(opts):
    assess_estimates_(get_milestones_(opts))


def main():
    parser = ArgumentParser(
        description="Get information about kcf style tasks. For more verbosity set the env var KCF_VERBOSITY to LOG."
    )
    subparsers = parser.add_subparsers(
        title="subcommands", description="valid subcommands", help="additional help"
    )

    parser.set_defaults(func=list_milestones)

    listtasks_parser = subparsers.add_parser("list-tasks")
    listtasks_parser.set_defaults(func=list_tasks)

    listmilestones_parser = subparsers.add_parser("list-milestones")
    listmilestones_parser.set_defaults(func=list_milestones)

    assess_estimates_parser = subparsers.add_parser("assess-estimates")
    assess_estimates_parser.set_defaults(func=assess_estimates)

    parser.add_argument(
        "--milestone",
        default="/",
        help="filter by milestone. You can filter by milestone group too by ending the milestone name with a /.",
    )
    parser.add_argument(
        "--complete",
        default=False,
        help="Only work with completed tasks.",
        action="store_true",
    )
    parser.add_argument(
        "--json", default=False, help="Format output as JSON.", action="store_true"
    )
    parser.add_argument(
        "--incomplete",
        default=False,
        help="Only work with incomplete tasks.",
        action="store_true",
    )

    parser.add_argument("source", nargs="*", default=[os.getcwd()])

    opts = parser.parse_args()

    opts.func(opts)


if __name__ == "__main__":
    main()