// Example Rhai plugin: always matches, returns one 'greet' target
// and builds a command to echo a greeting message.
fn name() {
    // Plugin name
    "rhai-test"
}

fn matches(dir) {
    // Always match any directory
    true
}

fn collect_targets(dir) {
    // Return a JSON array with a single target named 'greet'
    "[{\"name\":\"greet\",\"metadata\":null}]"
}

fn build_command(dir, target) {
    // Build a JSON CommandSpec: echo a greeting plus the target name
    let msg = "Hello from Rhai: " + target;
    "{\"prog\":\"echo\",\"args\":[\"" + msg + "\"],\"cwd\":null}"
}