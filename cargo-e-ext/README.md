# Rhai Plugin Examples for `cargo-e-ext`

This repository demonstrates how to write Rhai-based plugins for `cargo-e-ext` with full in-process execution, per-target overrides, and fallback to external commands.

## Plugins

### plugins/example.rhai
A minimal example plugin (`rhai-test`) that:
- Implements the required entrypoints:
  - `fn name() -> String`
  - `fn matches(dir: String) -> bool`
  - `fn collect_targets(dir: String) -> String` (returns JSON array of targets)
  - `fn build_command(dir: String, target: String) -> String` (returns JSON CommandSpec)
- Provides a single `greet` target that shells out via `echo`.

### plugins/demo_fallback.rhai
A demonstration plugin (`rhai-demo`) illustrating:
- Two targets: `inproc` and `external`.
  1. `inproc`: handled in-process by the generic `fn run(dir, target)`.
  2. `external`: falls back to spawning the external command defined in `build_command(dir, target)`.
- Optional per-target override: you can uncomment `fn external(...)` to see how per-target functions take priority.

## Running the Demos

1. Build the tool:
    ```bash
    cargo build
    ```

2. Run and select a target:
    ```bash
    cargo run
    ```
   Example session:
    ```text
    Available targets:
      0: [rhai-test] greet (plugins/example.rhai)
      1: [rhai-demo] inproc (plugins/demo_fallback.rhai)
      2: [rhai-demo] external (plugins/demo_fallback.rhai)
    Select a target by number: 1
    Running target 'inproc' from plugin 'rhai-demo'
    ✅  in-process handler for inproc
    ```

3. Test external fallback:
    ```text
    Select a target by number: 2
    Running target 'external' from plugin 'rhai-demo'
    🛑  external fallback for external
    ```

4. Test per-target override:
   - Uncomment the `fn external(dir, target)` definition in `plugins/demo_fallback.rhai`.
   - Rebuild and select target `external` again.
   - You should see the per-target array returned instead of the fallback.

## Writing Your Own Rhai Plugin

See `src/rhai_plugin.rs` and `plugins/example.rhai` for detailed comments. In summary, a Rhai plugin script (`*.rhai`) should provide:

- **Required:**
  - `fn name() -> String`: plugin identifier
  - `fn matches(dir: String) -> bool`: whether to apply in the current directory
  - `fn collect_targets(dir: String) -> String`: JSON array of `{ name, metadata }`
  - `fn build_command(dir: String, target: String) -> String`: JSON `CommandSpec` to spawn

- **Optional (in-process):**
  - `fn <target>(dir: String, target: String) -> Array`: per-target handler
  - `fn run(dir: String, target: String) -> Array`: generic handler

Host execution order:
1. Call per-target `fn <target>(dir, target)` if defined.
2. Else call generic `fn run(dir, target)` if defined.
3. Else spawn the system command from `build_command`.

Any in-process function must return an `Array` of `String`s, where:
- The first element is the exit code (e.g. "0").
- Remaining elements are output lines.