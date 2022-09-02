import os
from setuptools import setup, find_packages
import sys
import subprocess

here = os.path.abspath(os.path.dirname(__file__))
import codecs

requires = ["Django"]

version = subprocess.check_output(["git", "describe", "--abbrev=0", "--tags"]).decode(
    "utf-8"
)

setup(
    name="kcf-tasks",
    version=version,
    author="Timothy Hobbs",
    author_email="timothy@hobbs.cz",
    url="https://github.com/gradesta/gradesta/kcf",
    download_url="http://pypi.python.org/pypi/kcf-tasks/",
    description="Manage tasks in a Kanban Code Flow repo",
    long_description=codecs.open(os.path.join(here, "README.md"), "r", "utf-8").read(),
    license="CC0",
    packages=find_packages(),
    include_package_data=True,
    zip_safe=False,
    classifiers=[
        "Natural Language :: English",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3.10",
    ],
    scripts = ["kcf_tasks/kcf-tasks"],
)
