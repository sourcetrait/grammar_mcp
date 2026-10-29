# matrix.nu

## The module imports
Through constants built from the script's real path, as `build.nu`'s are, so
the script runs from any working directory.

## def main
The verdicts print as one row a profile and triple, a list of packages joined
to one cell, then the evidence directory; the exit is 1 when any check fails,
so a caller tells a failure by the status alone.
