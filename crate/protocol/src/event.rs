//! The worker's event stream, its cursors, and the backlog and Channel gate.
use crate::*;
use std::collections::VecDeque;

/// Where a subscriber stands: a generation and the last sequence it holds.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub struct EventCursor {
    pub generation: WorkerGeneration,
    pub sequence: u64,
}

/// What an event says.
#[cereal::derived(Data)]
pub enum EventPayload {
    Host(HostEvent),
    Emergency(Emergency),
    Lease(LeaseNotice),
    /// A remote link's lifecycle, or a send's delivery.
    Remote(RemoteChannelEvent),
    /// A peer's message, its files the record's artifacts.
    Relayed(RemotePacket),
}

/// An artifact an event refers to, and where its frontend delivers it.
#[cereal::derived(Data)]
pub struct EventArtifact {
    pub artifact: ArtifactRef,
    /// Its destination within the delivery, held to `check_dest`.
    pub dest: String,
}

/// One event as the worker numbered it.
#[cereal::derived(Data)]
pub struct EventRecord {
    pub generation: WorkerGeneration,
    pub sequence: u64,
    /// The Channel epoch it was emitted under, where it had one.
    pub channel: Option<ChannelEpoch>,
    pub payload: EventPayload,
    pub artifacts: Vec<EventArtifact>,
}

/// What an Events stream carries to its subscriber.
#[cereal::derived(Data)]
pub enum EventFrame {
    Record(Box<EventRecord>),
    /// The sequences from `from` to `to`, inclusive, dropped before they were read.
    Gap { from: u64, to: u64 },
    /// The worker restarted: the new generation and its oldest retained sequence.
    GenerationReset { generation: WorkerGeneration, oldest: u64 },
}

/// A subscription's start: after its cursor, or from the oldest retained.
#[cereal::derived(Data, Copy, Eq)]
pub struct EventResume {
    pub cursor: Option<EventCursor>,
}

/// A subscriber's receipt of every event up to its cursor, and nothing more.
#[cereal::derived(Data, Copy, Eq)]
pub struct EventAck {
    pub cursor: EventCursor,
}

/// What a subscriber at a cursor is served.
#[derive(Debug, Clone, PartialEq)]
pub enum Delivery {
    Frames(Vec<EventFrame>),
    /// Too far behind to be held: its stream is closed.
    Close { behind: u64 },
}

/// A generation's numbered events, held to the negotiated records and bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBacklog {
    generation: WorkerGeneration,
    /// The sequence the next record takes, from 1.
    next: u64,
    records: VecDeque<(EventRecord, u64)>,
    bytes: u64,
    max_records: u64,
    max_bytes: u64,
}

impl EventBacklog {
    /// A generation's backlog under the stream's negotiated `limits`.
    pub fn new(generation: WorkerGeneration, limits: &Limits) -> Self {
        Self {
            generation,
            next: 1,
            records: VecDeque::new(),
            bytes: 0,
            max_records: u64::from(limits.event_records),
            max_bytes: limits.event_bytes,
        }
    }

    /// The oldest sequence still held, or the next one while none is.
    pub fn oldest(&self) -> u64 {
        self.records.front().map_or(self.next, |(record, _)| record.sequence)
    }

    /// The newest sequence pushed, if any.
    pub fn newest(&self) -> Option<u64> {
        self.next.checked_sub(1).filter(|newest| *newest > 0)
    }

    /// An event numbered and held: its sequence, or refused.
    pub fn push(
        &mut self,
        channel: Option<ChannelEpoch>,
        payload: EventPayload,
        artifacts: Vec<EventArtifact>,
    ) -> Result<u64, TransportError> {
        match &payload {
            EventPayload::Relayed(packet) => packet.check()?,
            EventPayload::Host(host) if [&host.from, &host.model].iter().any(|s| s.starts_with(REMOTE_RESERVED_PREFIX)) => {
                return Err(TransportError::InvalidFrame {
                    reason: "a remote link's packet rides Remote or Relayed, never Host".to_string(),
                });
            }
            _ => {}
        }
        for artifact in &artifacts {
            check_dest(&artifact.dest).map_err(|e| TransportError::InvalidFrame { reason: e.message() })?;
        }
        let record = EventRecord { generation: self.generation, sequence: self.next, channel, payload, artifacts };
        let size = bitcode::encode(&EventFrame::Record(Box::new(record.clone()))).len() as u64;
        if size > u64::from(EVENT_FRAME_MAX) {
            return Err(TransportError::ResultTooLarge { limit: u64::from(EVENT_FRAME_MAX) });
        }
        self.records.push_back((record, size));
        self.bytes += size;
        while self.records.len() as u64 > self.max_records || self.bytes > self.max_bytes {
            let Some((_, dropped)) = self.records.pop_front() else { break };
            self.bytes -= dropped;
        }
        self.next += 1;
        Ok(self.next - 1)
    }

    /// The frames a subscriber after `cursor` reads, at most `max` records.
    pub fn read(&self, cursor: Option<EventCursor>, max: usize) -> Result<Vec<EventFrame>, TransportError> {
        let mut frames = Vec::new();
        let wanted = match cursor {
            None => self.oldest(),
            Some(cursor) if cursor.generation != self.generation => {
                frames.push(EventFrame::GenerationReset { generation: self.generation, oldest: self.oldest() });
                self.oldest()
            }
            Some(cursor) if cursor.sequence >= self.next => {
                return Err(TransportError::InvalidFrame {
                    reason: format!("a cursor at {} past the newest event", cursor.sequence),
                });
            }
            Some(cursor) => cursor.sequence + 1,
        };
        let oldest = self.oldest();
        if wanted < oldest {
            frames.push(EventFrame::Gap { from: wanted, to: oldest - 1 });
        }
        let from = wanted.max(oldest);
        frames.extend(
            self.records
                .iter()
                .filter(|(record, _)| record.sequence >= from)
                .take(max)
                .map(|(record, _)| EventFrame::Record(Box::new(record.clone()))),
        );
        Ok(frames)
    }

    /// A subscriber at `cursor` served, or closed past `window` events behind.
    pub fn deliver(&self, cursor: Option<EventCursor>, window: u64, max: usize) -> Result<Delivery, TransportError> {
        if let Some(cursor) = cursor.filter(|cursor| cursor.generation == self.generation) {
            let behind = self.newest().unwrap_or(0).saturating_sub(cursor.sequence);
            if behind > window {
                return Ok(Delivery::Close { behind });
            }
        }
        self.read(cursor, max).map(Delivery::Frames)
    }

    /// The receipt cursor after `ack`, from the subscriber's `acked`.
    pub fn ack(&self, acked: Option<EventCursor>, ack: EventAck) -> Result<EventCursor, TransportError> {
        if ack.cursor.generation != self.generation {
            return Err(TransportError::InvalidFrame { reason: "an ack of another generation's events".to_string() });
        }
        if ack.cursor.sequence >= self.next {
            return Err(TransportError::InvalidFrame { reason: "an ack past the newest event".to_string() });
        }
        Ok(match acked {
            Some(acked) if acked.generation == self.generation && acked.sequence > ack.cursor.sequence => acked,
            _ => ack.cursor,
        })
    }
}

/// The Channel's epoch and phase, its verification its own epoch's alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelGate {
    epoch: ChannelEpoch,
    phase: ChannelPhase,
    claimed: bool,
}

impl Default for ChannelGate {
    fn default() -> Self {
        Self { epoch: ChannelEpoch(0), phase: ChannelPhase::Closed, claimed: false }
    }
}

impl ChannelGate {
    /// The current epoch and phase.
    pub fn state(&self) -> (ChannelEpoch, ChannelPhase, bool) {
        (self.epoch, self.phase, self.claimed)
    }

    /// The epoch events are emitted under, while the Channel is verified.
    pub fn verified(&self) -> Option<ChannelEpoch> {
        (self.phase == ChannelPhase::Verified).then_some(self.epoch)
    }

    /// A new lease's reset: the Channel closed, nothing verified carried over.
    pub fn reset(&mut self) {
        self.phase = ChannelPhase::Closed;
        self.claimed = false;
    }

    /// A frontend's notice, refused unless it names the epoch it moves.
    pub fn notice(&mut self, notice: &ChannelNotice) -> Result<(), TransportError> {
        let refused = |reason: &str| Err(TransportError::InvalidFrame { reason: reason.to_string() });
        match notice {
            ChannelNotice::Opened { epoch, .. } if epoch.0 > self.epoch.0 => {
                *self = Self { epoch: *epoch, phase: ChannelPhase::Open, claimed: false };
                Ok(())
            }
            ChannelNotice::Opened { .. } => refused("an open of an epoch not newer than the Channel's"),
            ChannelNotice::Claimed { epoch } if *epoch == self.epoch && self.phase == ChannelPhase::Open => {
                self.claimed = true;
                Ok(())
            }
            ChannelNotice::Ready { epoch } if *epoch == self.epoch && self.phase == ChannelPhase::Open && self.claimed => {
                self.phase = ChannelPhase::Verified;
                Ok(())
            }
            ChannelNotice::Closed { epoch, .. } if *epoch == self.epoch => {
                self.reset();
                Ok(())
            }
            _ => refused("a Channel notice for another epoch, or out of its order"),
        }
    }
}
