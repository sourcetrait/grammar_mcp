# cli.rs

## enum ProbeCmd
The qualification contract's two commands, `inventory --out <path>` and `run
--case <case> --candidate-set <nuon> --fixture <nuon> --out <directory>`,
with its argument names, so a runner calls the probe the way the contract
writes it. The fixture is the run's redacted identity manifest.
