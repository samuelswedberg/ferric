use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::RepeatMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub enum Command {
    PlayPause,
    Next,
    Previous,
    Seek {
        #[ts(type = "number")]
        position_ms: u64,
    },
    SetVolume {
        pct: u8,
    },
    SetLiked {
        liked: bool,
    },
    SetPlaybackMode {
        shuffle: bool,
        repeat: RepeatMode,
    },
    JumpTo {
        queue_index: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::model::RepeatMode;
    use serde_json::{json, Value};

    fn assert_round_trip(command: Command, expected: Value) {
        let serialized = serde_json::to_value(&command).expect("serialization should succeed");
        assert_eq!(serialized, expected);
        let deserialized: Command =
            serde_json::from_value(serialized).expect("deserialization should succeed");
        assert_eq!(deserialized, command);
    }

    #[test]
    fn variants_match_ipc_json_shapes() {
        assert_round_trip(Command::PlayPause, json!({"type": "playPause"}));
        assert_round_trip(Command::Next, json!({"type": "next"}));
        assert_round_trip(Command::Previous, json!({"type": "previous"}));
        assert_round_trip(
            Command::Seek { position_ms: 1234 },
            json!({"type": "seek", "positionMs": 1234}),
        );
        assert_round_trip(
            Command::SetVolume { pct: 75 },
            json!({"type": "setVolume", "pct": 75}),
        );
        assert_round_trip(
            Command::SetLiked { liked: true },
            json!({"type": "setLiked", "liked": true}),
        );
        assert_round_trip(
            Command::SetPlaybackMode {
                shuffle: true,
                repeat: RepeatMode::Context,
            },
            json!({"type": "setPlaybackMode", "shuffle": true, "repeat": "context"}),
        );
        assert_round_trip(
            Command::JumpTo { queue_index: 3 },
            json!({"type": "jumpTo", "queueIndex": 3}),
        );
    }
}
