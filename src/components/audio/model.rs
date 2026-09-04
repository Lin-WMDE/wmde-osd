// Copyright 2026 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use cosmic_settings_audio_client::{self as audio_client};

pub type NodeId = u32;

#[derive(Debug, Default)]
pub struct Model {
    sinks: Nodes,
    sources: Nodes,
    pub active_sink: ActiveNode,
    pub active_source: ActiveNode,
    default_sink: Option<NodeId>,
    default_source: Option<NodeId>,
}

/// Audio nodes of one direction.
///
/// Volume and mute stay `None` until the daemon reports them. `Event::Node`
/// announces a node and carries neither, so there is no placeholder to mistake
/// for a reading: see [`Model::sink_changed`].
#[derive(Debug, Default)]
pub struct Nodes {
    active: Option<usize>,
    mute: Vec<Option<bool>>,
    id: Vec<NodeId>,
    volume: Vec<Option<u32>>,
}

impl Nodes {
    fn position(&self, node_id: NodeId) -> Option<usize> {
        self.id.iter().position(|id| node_id == *id)
    }

    /// Register a node, or report where it already sits.
    fn insert(&mut self, node_id: NodeId) -> usize {
        if let Some(pos) = self.position(node_id) {
            return pos;
        }
        self.id.push(node_id);
        self.volume.push(None);
        self.mute.push(None);
        self.id.len() - 1
    }

    /// Volume and mute of the active node, once the daemon has reported both.
    fn active_state(&self) -> Option<(u32, bool)> {
        let pos = self.active?;
        let volume = self.volume.get(pos).copied().flatten()?;
        let mute = self.mute.get(pos).copied().flatten()?;
        Some((volume, mute))
    }

    pub fn remove(&mut self, node_id: u32) -> bool {
        let Some(pos) = self.position(node_id) else {
            return false;
        };
        self.mute.remove(pos);
        self.id.remove(pos);
        self.volume.remove(pos);
        // Everything behind the removed node shifts one place down, and the
        // active index with it.
        self.active = match self.active {
            Some(active) if active == pos => None,
            Some(active) if active > pos => Some(active - 1),
            active => active,
        };
        true
    }
}

#[derive(Debug, Default)]
pub struct ActiveNode {
    pub volume: u32,
    pub mute: bool,
    /// Whether the two fields above hold a state the daemon actually reported.
    seen: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Response {
    SinkVolume(u32, bool),
    SourceVolume(u32, bool),
}

impl Model {
    pub fn update(&mut self, event: audio_client::Event) -> Option<Response> {
        match event {
            audio_client::Event::NodeMute(node_id, mute) => {
                if let Some(pos) = self.sinks.position(node_id) {
                    self.sinks.mute[pos] = Some(mute);
                    if self.sinks.active == Some(pos) {
                        return self.sink_changed();
                    }
                } else if let Some(pos) = self.sources.position(node_id) {
                    self.sources.mute[pos] = Some(mute);
                    if self.sources.active == Some(pos) {
                        return self.source_changed();
                    }
                }
            }

            audio_client::Event::NodeVolume(node_id, volume, _balance) => {
                if let Some(pos) = self.sinks.position(node_id) {
                    self.sinks.volume[pos] = Some(volume);
                    if self.sinks.active == Some(pos) {
                        return self.sink_changed();
                    }
                } else if let Some(pos) = self.sources.position(node_id) {
                    self.sources.volume[pos] = Some(volume);
                    if self.sources.active == Some(pos) {
                        return self.source_changed();
                    }
                }
            }

            audio_client::Event::DefaultSink(node_id) => {
                self.default_sink = Some(node_id);
                self.sinks.active = self.sinks.position(node_id);
                return self.sink_changed();
            }

            audio_client::Event::DefaultSource(node_id) => {
                self.default_source = Some(node_id);
                self.sources.active = self.sources.position(node_id);
                return self.source_changed();
            }

            audio_client::Event::Node(node_id, node) => {
                if node.is_sink {
                    let pos = self.sinks.insert(node_id);
                    if self.default_sink == Some(node_id) {
                        self.sinks.active = Some(pos);
                        return self.sink_changed();
                    }
                } else {
                    let pos = self.sources.insert(node_id);
                    if self.default_source == Some(node_id) {
                        self.sources.active = Some(pos);
                        return self.source_changed();
                    }
                }
            }

            audio_client::Event::RemoveNode(node_id) => {
                if !self.sinks.remove(node_id) {
                    self.sources.remove(node_id);
                }
            }

            _ => (),
        }

        None
    }

    /// Adopt the state of the active sink, and report a change to the caller,
    /// which raises the OSD.
    ///
    /// The first complete state is adopted in silence, because it is knowledge,
    /// not a change: the daemon replays everything it knows to every client that
    /// subscribes, and that burst arrives at session start and again after every
    /// reconnect. Only a difference against a state already adopted raises the
    /// indicator.
    fn sink_changed(&mut self) -> Option<Response> {
        let (volume, mute) = self.sinks.active_state()?;
        let changed = self.active_sink.seen
            && (self.active_sink.volume != volume || self.active_sink.mute != mute);
        self.active_sink = ActiveNode {
            volume,
            mute,
            seen: true,
        };
        changed.then_some(Response::SinkVolume(volume, mute))
    }

    /// The same for the active source. See [`Model::sink_changed`].
    fn source_changed(&mut self) -> Option<Response> {
        let (volume, mute) = self.sources.active_state()?;
        let changed = self.active_source.seen
            && (self.active_source.volume != volume || self.active_source.mute != mute);
        self.active_source = ActiveNode {
            volume,
            mute,
            seen: true,
        };
        changed.then_some(Response::SourceVolume(volume, mute))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_client::{Event, NodeInfo};

    const SINK: NodeId = 1;
    const SOURCE: NodeId = 2;

    fn node(is_sink: bool) -> NodeInfo {
        NodeInfo {
            name: String::new(),
            description: String::new(),
            device_profile_description: String::new(),
            device_id: None,
            card_profile_device: None,
            is_sink,
        }
    }

    /// What the daemon replays to every client that subscribes, in its order:
    /// `wmde-settings-daemon/audio-server/src/backend.rs`.
    fn subscribe() -> Vec<Event> {
        vec![
            Event::Node(SINK, node(true)),
            Event::Node(SOURCE, node(false)),
            Event::DefaultSink(SINK),
            Event::DefaultSource(SOURCE),
            Event::NodeVolume(SINK, 55, None),
            Event::NodeVolume(SOURCE, 40, None),
            Event::NodeMute(SINK, false),
            Event::NodeMute(SOURCE, true),
        ]
    }

    fn connected() -> Model {
        let mut model = Model::default();
        for event in subscribe() {
            assert_eq!(
                model.update(event),
                None,
                "the state replayed on subscribe is knowledge, not a change",
            );
        }
        model
    }

    #[test]
    fn subscribing_raises_no_indicator() {
        let model = connected();
        assert_eq!(model.active_sink.volume, 55);
        assert!(!model.active_sink.mute);
        assert_eq!(model.active_source.volume, 40);
        assert!(model.active_source.mute);
    }

    #[test]
    fn volume_key_raises_the_indicator() {
        let mut model = connected();
        assert_eq!(
            model.update(Event::NodeVolume(SINK, 60, None)),
            Some(Response::SinkVolume(60, false)),
        );
    }

    #[test]
    fn microphone_mute_raises_the_indicator() {
        let mut model = connected();
        assert_eq!(
            model.update(Event::NodeMute(SOURCE, false)),
            Some(Response::SourceVolume(40, false)),
        );
    }

    #[test]
    fn repeating_a_value_stays_quiet() {
        let mut model = connected();
        assert_eq!(model.update(Event::NodeVolume(SINK, 55, None)), None);
        assert_eq!(model.update(Event::NodeMute(SINK, false)), None);
    }

    #[test]
    fn a_node_that_is_not_default_stays_quiet() {
        let mut model = connected();
        model.update(Event::Node(3, node(true)));
        assert_eq!(model.update(Event::NodeVolume(3, 90, None)), None);
        assert_eq!(model.update(Event::NodeMute(3, true)), None);
    }

    #[test]
    fn switching_the_default_raises_the_indicator() {
        let mut model = connected();
        model.update(Event::Node(3, node(true)));
        model.update(Event::NodeVolume(3, 90, None));
        model.update(Event::NodeMute(3, false));
        assert_eq!(
            model.update(Event::DefaultSink(3)),
            Some(Response::SinkVolume(90, false)),
        );
    }

    #[test]
    fn removing_a_node_keeps_the_active_one() {
        let mut model = connected();
        model.update(Event::Node(3, node(true)));
        model.update(Event::NodeVolume(3, 90, None));
        model.update(Event::NodeMute(3, false));
        model.update(Event::DefaultSink(3));

        // The removed node sits ahead of the active one in the list.
        model.update(Event::RemoveNode(SINK));

        assert_eq!(
            model.update(Event::NodeVolume(3, 95, None)),
            Some(Response::SinkVolume(95, false)),
            "the active index must follow the node, not its old position",
        );
    }
}
