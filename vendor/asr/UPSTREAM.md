# Source

LiveSplit/asr, commit `89d55ab07198da6fd75cab1ea1a6825b4240b343`.
https://github.com/LiveSplit/asr/tree/89d55ab07198da6fd75cab1ea1a6825b4240b343

Vendored with original MIT/Apache licenses. Local change in
`src/game_engine/unity/managed/walk.rs`: apply the Unity parent-climb stop
only after the explicitly requested initial class. This allows resolving
`UnityEngine.Object.m_CachedPtr` by metadata without hard-coding its offset.
Searching this field through Game still stops at Unity base classes as before.
Verified against live Unity 6000.3.6f1: Object resolves offset 16, Game returns None.
