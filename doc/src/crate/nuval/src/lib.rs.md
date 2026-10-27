# lib.rs

## use sourcetrait_common::cereal
cereal comes through the workspace's `sourcetrait_common`, its facade feature
set, rather than `sourcetrait_cereal` alone (User). `cereal::derived` emits
absolute `::serde`, `::bitcode`, and `::rkyv` paths, so those three are the
crate's own dependencies, at the instances cereal uses.

## mod nu
nushell's items the conversions name, under one path, so a reader tells them
from nuval's own.
