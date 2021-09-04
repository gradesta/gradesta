#!/usr/bin/python3
import argparse
import asyncio
import capnp
import yaml
import zmq
import zmq.asyncio

import level0_capnp as level0

parser = argparse.ArgumentParser(
    description="""
Low level tester for the gradesta protocol. Allows you to interact with gradesta services/clients by sending / viewing capnp messages directly.

You operate the tool by loading a yaml replay file. This file is a stream of yaml documents. Each document should contain a dictionary consisting of one or more records. There are three types of records supported:

---
expect: <level0-yaml-message-match-expression>
---
drop: <num-messages-to-drop>
---
send: <level0-yaml-message>

For information on level0-yaml see the level0_yaml.py tool.
"""
)
parser.add_argument("socket", metavar="socket", type=str, help="socket to connect to")
parser.add_argument("replay", metavar="replay", type=str, help="replay yaml file")

args = parser.parse_args()

context = zmq.asyncio.Context()
socket = context.socket(zmq.REP)
socket.bind(args.socket)
