# Modern AlChemy – Current Issues (fontanagen branch)

##### How to Run
- **Build:** `cargo build`
- **Run default simulation:** `cargo run`
- **Fontana generator demo:** `cargo run -- --config-file /path/to/fontana_config.json`
- **Dump default config:** `cargo run -- --dump-config`
- **Dump default config with CL:** cargo run -- --config-file /tmp/fontana_cfg.json --generate 3
- **Run experiments:** `cargo run -- --experiment <name>` (not sure how to run this part)

##### Currently Resolved

##### Fontana generator looks at configuration
- `src/generators.rs`: `FontanaGen::from_config` now uses `min_depth`, `max_depth`, and `free_variable_probability`, clamps probability ranges, checks depth bounds, and removed the unused local RNG binding.
- Added a defensive helpers to avoid divide-by-zero (`max_depth` guard), empty ranges when depth is zero, and runaway probabilities (clamped per depth). Generation can now use the configured `n_max_free_vars` safely.
- CLI `--generate` now obeys the active generator in the loaded config, so Fontana samples can be printed without editing source. 

##### Previous issue: 

- `src/generators.rs:202-217`: `min_depth` and `free_prob` are hard-coded to `0`, ignoring `config::FontanaGen.min_depth` and `.free_variable_probability`. (but it seems like in config.rs it matches the same values? do we hard code or use that?)
- `src/generators.rs:197-200`: the local `rng` binding is never used; we instantiate the struct with `ChaCha8Rng::from_seed(seed)` directly, so this variable only triggers warnings.
- `src/generators.rs:197-200`: division by `cfg.max_depth - 1` won't work if `max_depth <= 1`.
- `src/generators.rs:236-279`: `p_abs`/`p_app` can exceed `1.0`, meaning leaf nodes might never be produced before hitting maximum depth; when `depth == 0`, `rng.gen_range(1..=depth)` is an empty range.
- `src/main.rs:156`: `--generate` always uses `BTreeGen`. Need to do for FontanaGen


##### Warning cleanup
- Removed unused helper functions and tightened imports in `src/experiments/magic_test_function.rs`; wrapped tests in `#[cfg(test)]` to avoid dev-build noise.
- Added documentation for `config::FontanaGen`, so the `#[warn(missing_docs)]` lint is satisfied.
- `cargo build` and `cargo test` now run warning-free.

##### Remaining Known Issues

##### Postfix standardization unimplemented
- `src/generators.rs:134-136`: `postfix_standardize` calls `unimplemented!`.
- Any config selecting `Standardization::Postfix` crashes.
- Need to add the transformation or disable the option in configs/CLI until ready.

##### Collision metrics mislabelled
- `src/lambda/recursive.rs:227-232`: tuples are pushed as `(expr, size, reductions)` but collected as reductions → `t.1`, sizes → `t.2`, swapping the data.
- **Does this imapct antyhing? does it affect the analytics and experiment outputs report incorrect reduction counts vs sizes.

##### Jacard metric incorrect
- `src/analysis.rs:57-68`: calculates `intersection / (|A| + |B|)`
- should be divded by A U B

##### Documentation
- `scripts/discovery.sh:15` (and similar files): `cd ~/cwd/functional-supercollider`, which doesn’t exist here.


