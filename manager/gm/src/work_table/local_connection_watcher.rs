/*
This is the source code for Gradesta's local connection watcher.

The watcher will watch `$GRADESTA_USER_SOCKET_DIR` or if not set `~/.cache/gradesta/services/`.
A common value for `$GRADESTA_USER_SOCKET_DIR` would be `/var/run/user/<uid>/gradesta/services/`
if your distro supports `/var/run/user/<uid>/`.

When the watcher is initialized the first thing it does is print its loading message to `stdout` as a yaml document.

```
- status: loading
  version: yyy-mm v
```

check `/proc/net/unix`
to see if there are currently running gradesta services.
If there are, it shows an error message and reports their PIDs to `stderr` and exits.

The format for `stderr` is a yaml document of the form:

- error: There are gradesta services currently running. Either the manager is already launched or you need to terminate these services before launching the manager.
  pids: [343, 434, 857]

If there are left over sockets not in `/proc/net/unix`,
the watcher will delete these sockets and report that action to `stdout` in the yaml format

- info: Deleting left over sockets.
  sockets: ["/home/user/.cache/gradesta/services/service_name/PAIR.zmq", "/home/user/.cache/gradesta/services/service_name2/PAIR.zmq"]
  note: Launching services during gradesta-manager/watcher startup may lead to race conditions. Always wait for ready or use the gradesta-init-system to launch services.
---

Once this step is completed, the ready status will be sent to `stdout`.

- status: ready



*/
