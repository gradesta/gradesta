log-reading-first-level-dir-entry = Reading first level dir entry. { $dir }{ $dir }
could-not-read-dir = { $err_code }: Could not read dir { $dir }
 { $error }
error-looking-up-socket-information = { $err_code }: Error looking up socket information for socket { $socket }
 { $error }

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



