could-not-remove-dir = { $err_code }: Could not remove dir { $dir }
 { $error }
log-reading-first-level-dir-entry = Reading first level dir entry. { $dir }{ $dir }
could-not-read-dir = { $err_code }: Could not read dir { $dir }
 { $error }
error-looking-up-socket-information = { $err_code }: Error looking up socket information for socket { $socket }
 { $error }
watch-error = { $err_code }: Watch error
 { $error }
sockets-still-in-use = { $err_code }: Could not start because the following gradesta service sockets are still in use:
 { $error }

process-listing-no-longer-running = "    { $pid } - no longer running"
process-listing-socket-header = "{ $socket }\n    PID - name\n
process-listing-socket-in-use =
  The socket { $socket } is currently in use by the following program
      PID - name
      { $pid }
  Would you like to [t term, k kill, i ignore, w wait 1 sec] this process?
process-listing-unkown-option = Unknown option

gradesta-command = gradesta manager
gradesta-command-about = Connects clients and services via websockets and ZMQ unix PAIR sockets. Evaluates walk trees.
gradesta-command-sockets-dir-help = Directory where ZMQ unix sockets are found
gradesta-command-init-help = Init binary for launching services after manager startup
gradesta-command-port-help = Port for websockets to connect
gradesta-command-no-websockets-help = Dissable websockets
gradesta-command-no-unix-sockets-help = Dissable unix sockets
gradesta-command-service-heartbeat-timeout-help = Amount of time to wait after heartbeat before closing connection
gradesta-command-no-sockets-error = { $err_code }: Either UNIX sockets or websockets must be enabled.

lock-sockets-dir-open-lock-error = { $err_code }: Error opening lock file { $file }: { $error }
lock-sockets-dir-lock-error = { $err_code }: Error locking lock file { $file }: { $error }
lock-sockets-dir-read-error = { $err_code }: Error reading lock file { $file }: { $error }
lock-sockets-dir-parse-error = { $err_code }: Error parsing lock file { $file }: { $error }
lock-sockets-dir-already-locked = { $err_code }: The sockets directory is already locked by pid { $pid } { $process_name }
lock-sockets-dir-write-error = { $err_code }: Error writing lock file { $file }: { $error }
lock-sockets-dir-unlock-error = { $err_code }: Error unlocking lock file { $file }: { $error }

# Test cases for testing the fluent system
test_case_simple = Hello world!
test_case_intro = Welcome, { $name }.
test_case_plural = { $num ->
    [one] You have { $num } new message.
    [3] You have a few new messages.
   *[other] You have { $num } new messages.
}
test_case_two_params = 1. { $one } 2. { $two }
test_case_three_params = 1. { $one } 2. { $two } 3. { $three }



