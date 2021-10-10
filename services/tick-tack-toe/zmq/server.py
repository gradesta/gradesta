import time
import zmq

context = zmq.Context()

# Socket to receive messages on
receiver = context.socket(zmq.PULL)
receiver.connect("ipc:///home/timothy/.cache/gradesta/services/sockets/push")


def print_value(s):
    time.sleep(2)
    print('value: {}'.format(s))


# Process tasks forever
while True:
    s = receiver.recv().decode("utf8")
    print('received {}'.format(s))
    print_value(s)
