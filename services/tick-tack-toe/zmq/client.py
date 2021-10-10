import zmq

context = zmq.Context()

# Socket to send messages on
sender = context.socket(zmq.PUSH)
sender.bind("ipc:///home/timothy/.cache/gradesta/services/sockets/push")

for i in range(3):
    print('sending {}'.format(i))
    sender.send(str(i).encode("utf8"))
