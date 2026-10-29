# output.rs

## fn write_json
The file is created with `create_new`, so an existing inventory or result is
refused rather than replaced: a result is evidence, and a rerun into the same
directory would otherwise rewrite what an earlier run recorded. The refusal is
a configuration failure, the runner's to fix by naming a fresh output.
