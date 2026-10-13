// USER_DATA set on one participant's entities must be readable by a remote participant.

use std::{
    thread::sleep,
    time::{Duration as StdDuration, Instant},
};

use crate::common::*;
use int2dds::{
    common::instance_handle::InstanceHandle,
    domain::{
        domain_participant::DomainParticipant,
        domain_participant_factory::DomainParticipantFactory, qos::DomainParticipantQos,
    },
    infrastructure::{qos_policy::UserDataQosPolicy, status::StatusMask},
    publication::qos::{DataWriterQos, PublisherQos},
    subscription::qos::{DataReaderQos, SubscriberQos},
};

const DISCOVERY_TIMEOUT: StdDuration = StdDuration::from_secs(10);

const POLL_INTERVAL: StdDuration = StdDuration::from_millis(50);

fn create_participant(domain_id: i32, qos: DomainParticipantQos) -> DomainParticipant {
    DomainParticipantFactory::get_instance()
        .create_participant(domain_id, qos, None, StatusMask::default())
        .unwrap()
}

fn delete_participant(participant: DomainParticipant) {
    participant.delete_contained_entities().unwrap();
    DomainParticipantFactory::get_instance().delete_participant(participant).unwrap();
}

// Polls until `find` returns Some or the deadline passes.
fn wait_for<T>(mut find: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + DISCOVERY_TIMEOUT;

    while Instant::now() < deadline {
        if let Some(found) = find() {
            return Some(found);
        }
        sleep(POLL_INTERVAL);
    }

    None
}

#[test]
fn participant_user_data_is_visible_to_remote_participant() {
    let domain_id = next_domain_id();
    let user_data = b"participant-user-data".to_vec();

    let sending_participant = create_participant(
        domain_id,
        DomainParticipantQos {
            user_data: UserDataQosPolicy { value: user_data.clone() },
            ..Default::default()
        },
    );
    let receiving_participant = create_participant(domain_id, DomainParticipantQos::default());

    let sending_handle = sending_participant.get_instance_handle().unwrap();

    let is_discovered = wait_for(|| {
        let handles = receiving_participant.get_discovered_participants().ok()?;
        handles.contains(&sending_handle).then_some(())
    });
    assert!(is_discovered.is_some(), "sending participant was not discovered");

    let participant_data =
        receiving_participant.get_discovered_participant_data(sending_handle).unwrap();
    assert_eq!(participant_data.user_data().value, user_data);

    delete_participant(sending_participant);
    delete_participant(receiving_participant);
}

#[test]
fn datawriter_user_data_is_visible_to_remote_datareader() {
    let domain_id = next_domain_id();
    let user_data = b"datawriter-user-data".to_vec();

    let writer_participant = create_participant(domain_id, DomainParticipantQos::default());
    let reader_participant = create_participant(domain_id, DomainParticipantQos::default());

    let data_writer = create_datawriter(
        &writer_participant,
        PublisherQos::default(),
        DataWriterQos {
            user_data: UserDataQosPolicy { value: user_data.clone() },
            ..Default::default()
        },
    );
    let data_reader =
        create_datareader(&reader_participant, SubscriberQos::default(), DataReaderQos::default());

    let writer_handle = data_writer.get_instance_handle().unwrap();

    let publication_handle: Option<InstanceHandle> = wait_for(|| {
        let handles = data_reader.get_matched_publications().ok()?;
        handles.into_iter().find(|handle| *handle == writer_handle)
    });
    let publication_handle = publication_handle.expect("datawriter was not matched");

    let publication_data = data_reader.get_matched_publication_data(publication_handle).unwrap();
    assert_eq!(publication_data.user_data().value, user_data);

    drop(data_writer);
    drop(data_reader);
    delete_participant(writer_participant);
    delete_participant(reader_participant);
}

#[test]
fn datareader_user_data_is_visible_to_remote_datawriter() {
    let domain_id = next_domain_id();
    let user_data = b"datareader-user-data".to_vec();

    let writer_participant = create_participant(domain_id, DomainParticipantQos::default());
    let reader_participant = create_participant(domain_id, DomainParticipantQos::default());

    let data_writer =
        create_datawriter(&writer_participant, PublisherQos::default(), DataWriterQos::default());
    let data_reader = create_datareader(
        &reader_participant,
        SubscriberQos::default(),
        DataReaderQos {
            user_data: UserDataQosPolicy { value: user_data.clone() },
            ..Default::default()
        },
    );

    let reader_handle = data_reader.get_instance_handle().unwrap();

    let subscription_handle: Option<InstanceHandle> = wait_for(|| {
        let handles = data_writer.get_matched_subscriptions().ok()?;
        handles.into_iter().find(|handle| *handle == reader_handle)
    });
    let subscription_handle = subscription_handle.expect("datareader was not matched");

    let subscription_data = data_writer.get_matched_subscription_data(subscription_handle).unwrap();
    assert_eq!(subscription_data.user_data().value, user_data);

    drop(data_writer);
    drop(data_reader);
    delete_participant(writer_participant);
    delete_participant(reader_participant);
}
