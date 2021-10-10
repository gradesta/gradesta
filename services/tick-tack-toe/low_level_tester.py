#!/usr/bin/python3
import argparse
#import asyncio
import capnp
import yaml
import zmq
#import zmq.asyncio

import level0_capnp as level0
import level0_yaml

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

#######################################################š
#######################################################š
#######################################################š
"""
Collors for colloring text in an ANSI terminal.
The list is taken from https://stackoverflow.com/questions/287871/print-in-terminal-with-colors-using-python
"""
HEADER = '\033[95m'
OKBLUE = '\033[94m'
OKGREEN = '\033[92m'
WARNING = '\033[93m'
FAIL = '\033[91m'
ENDC = '\033[0m'
BOLD = '\033[1m'
UNDERLINE = '\033[4m'
#######################################################š
#######################################################š
#######################################################š

context = zmq.Context()
push_socket = context.socket(zmq.PULL)
pull_socket = context.socket(zmq.PUSH)
push_socket_path = args.socket+".push"
print("Connecting to", push_socket_path)
push_socket.connect(push_socket_path)
pull_socket_path = args.socket+".pull"
print("Connecting to", pull_socket_path)
pull_socket.connect(pull_socket_path)
with open(args.replay) as fd:
    replay = yaml.load_all(fd)
    for move in replay:
        print("########################################################")
        if "expect" in move:
            print("Expecting")
            print(OKGREEN)
            print(move["expect"])
            print(ENDC)
            message = level0_capnp.Message.from_bytes(push_socket.recv())
            myml = level0_yaml.to_yaml(message)
            diff = level0.yaml.compare(move["expect"], myml)
            if diff:
                print(FAIL)
                print(myml)
                print(ENDC)
        if "drop" in move:
            for n in range(0, move["drop"]):
                print("Dropping message ", n, " of ", move["drop"])
                message = level0.Message.from_bytes(push_socket.recv())
                print(OKGREEN)
                myml = level0_yaml.to_dict(message)
                print(myml)
                print(ENDC)
        if "send" in move:
            message = level0_yaml.from_dict_to_capnp(move["send"]).to_bytes()
            print("Sending")
            print(OKGREEN)
            print(yaml.dump(move["send"]))
            print(ENDC)
            pull_socket.send(message)
            print("Sent")
