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
process-listing = ...
  .no-longer-running = "    { $pid } - no longer running"
  .socket-header = "{ $socket }\n    PID - name\n
  .socket-in-use = The socket { $socket } is currently in use by the following program
  PID - name
  { $pid }
  Would you like to [t term, k kill, i ignore, w wait 1 sec] this process?
  .unkown-option = Unknown option

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



