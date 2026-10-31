//! The face's hub slot: the verify timer, the hub's lanes, and the shutdown's ack.
use crate::*;
use sourcetrait_grammar_protocol::ChannelSpamWarningEmergency;

const EPOCH: ChannelEpoch = ChannelEpoch(1);

/// A hub-shaped slot entry without a hub: its senders, the receivers held here.
fn installed(
    slot: &HubSlot,
    epoch: ChannelEpoch,
) -> (
    tk::UnboundedReceiver<String>,
    tk::oneshot::Receiver<ChannelCloseSignal>,
    tk::oneshot::Receiver<()>,
) {
    let (packets, packet_rx) = tk::unbounded_channel::<String>();
    let (close, close_rx) = tk::oneshot::channel::<ChannelCloseSignal>();
    let (shutdown, shutdown_rx) = tk::oneshot::channel::<()>();
    slot.install(Hub {
        epoch,
        url: "wss://127.0.0.1:1".to_string(),
        packets,
        close,
        _shutdown: shutdown,
    });
    (packet_rx, close_rx, shutdown_rx)
}

#[test]
fn each_open_takes_a_fresh_epoch() {
    let slot = HubSlot::default();
    let first = slot.next_epoch();
    let second = slot.next_epoch();
    assert_ne!(first, second);
    assert_ne!(first, ChannelEpoch(0), "the Channel's own closed epoch is never minted");
}

#[test]
fn a_line_reaches_only_the_hub_of_its_open() {
    let slot = HubSlot::default();
    let (mut packets, _close, _shutdown) = installed(&slot, EPOCH);
    slot.deliver(ChannelEpoch(9), "stale".to_string());
    slot.deliver(EPOCH, "x".to_string());
    assert_eq!(packets.try_recv().ok().as_deref(), Some("x"), "the stale line was dropped");
    assert!(packets.try_recv().is_err());
    assert_eq!(slot.url(EPOCH).as_deref(), Some("wss://127.0.0.1:1"));
    assert_eq!(slot.url(ChannelEpoch(9)), None);
}

#[test]
fn a_gone_peer_shows_as_its_lane_closing() {
    let slot = HubSlot::default();
    let (packets, _close, _shutdown) = installed(&slot, EPOCH);
    assert!(!slot.peer_gone(EPOCH));
    drop(packets);
    assert!(
        slot.peer_gone(EPOCH),
        "the peer task drops its lane when it ends - that is how channel_open learns",
    );
    assert!(slot.peer_gone(ChannelEpoch(9)), "an open with no hub here has no peer");
}

#[test]
fn a_hub_close_signals_on_its_own_lane_and_ends_the_accept_loop() {
    let slot = HubSlot::default();
    let (_packets, mut close, mut shutdown) = installed(&slot, EPOCH);
    slot.close(ChannelEpoch(9), 1000, "stale".to_string());
    assert!(close.try_recv().is_err(), "a close for another open leaves this hub");
    slot.close(EPOCH, 1000, "done".to_string());
    let (code, reason, done) = close.try_recv().expect("the close rides its own lane");
    assert_eq!(
        (code, reason.as_str()),
        (1000, "done"),
        "a planned close must never wait behind traffic, or the agent sees a bare 1006",
    );
    assert!(done.is_none(), "a plain close asks for no flush ack");
    assert!(
        matches!(shutdown.try_recv(), Err(tk::oneshot::error::TryRecvError::Closed)),
        "the hub's shutdown sender dropped with it, which ends the accept loop",
    );
    assert_eq!(slot.url(EPOCH), None, "the slot is empty after");
}

#[test]
fn a_shutdown_close_hands_back_a_flush_ack() {
    let slot = HubSlot::default();
    let (_packets, mut close, _shutdown) = installed(&slot, EPOCH);
    let mut done_rx = slot
        .close_now(1001, "host shutting down")
        .expect("an installed hub has something to close");
    let (code, reason, done_tx) = close.try_recv().expect("the close was signalled");
    assert_eq!((code, reason.as_str()), (1001, "host shutting down"));
    done_tx
        .expect("a shutdown close carries an ack sender")
        .send(())
        .expect("ack");
    assert!(done_rx.try_recv().is_ok(), "the waiter observes the flush");
    assert!(
        slot.close_now(1001, "host shutting down").is_none(),
        "no hub means no frame to wait for, so shutdown must not block on one",
    );
}

#[test]
fn arming_the_timer_again_stands_the_previous_one_down() {
    let slot = HubSlot::default();
    let (first, mut first_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, first);
    assert!(
        matches!(first_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Empty)),
        "still armed while the slot holds the sender",
    );
    let (second, _second_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(ChannelEpoch(2), second);
    assert!(
        matches!(first_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Closed)),
        "re-opening drops the previous sender, so two timers never race one channel",
    );
}

#[test]
fn verification_and_a_hub_close_stand_the_timer_down_for_their_open_alone() {
    let slot = HubSlot::default();
    let (cancel, mut cancel_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, cancel);
    slot.disarm_verify(ChannelEpoch(9));
    assert!(
        matches!(cancel_rx.try_recv(), Err(tk::oneshot::error::TryRecvError::Empty)),
        "another open's verification leaves this timer armed",
    );
    slot.disarm_verify(EPOCH);
    assert!(matches!(
        cancel_rx.try_recv(),
        Err(tk::oneshot::error::TryRecvError::Closed),
    ));

    let (_packets, _close, _shutdown) = installed(&slot, EPOCH);
    let (cancel, mut cancel_rx) = tk::oneshot::channel::<()>();
    slot.arm_verify(EPOCH, cancel);
    slot.close(EPOCH, 1000, "done".to_string());
    assert!(matches!(
        cancel_rx.try_recv(),
        Err(tk::oneshot::error::TryRecvError::Closed),
    ));
}

#[test]
fn an_emergency_reaches_the_lane_once_one_is_installed() {
    let slot = HubSlot::default();
    let warning = Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
        origin: "thread/a".to_string(),
        hits: 3,
        window_secs: 10,
        rate: 3,
    });
    slot.raise(warning.clone());
    let (tx, mut rx) = tk::unbounded_channel::<Emergency>();
    slot.install_emergency(tx);
    slot.raise(warning.clone());
    assert_eq!(rx.try_recv().ok(), Some(warning), "raised after the install");
    assert!(rx.try_recv().is_err(), "the one raised before it went nowhere");
}
