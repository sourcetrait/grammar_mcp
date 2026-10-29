# json.rs

## fn json_value_to_nu_value
The write half of the rig index's NUON round-trip and the serde bridge every
tool's record crosses, on the engine and on the client alike, which is why it
sits behind nuval's `nu` feature rather than in the engine: the client builds
nuval with `nu` and no engine. An out-of-range number degrades to a string
rather than failing, on the reasoning that a value the agent sent is better
preserved lossily than rejected.
