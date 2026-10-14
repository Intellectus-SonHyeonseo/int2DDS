// USER_DATA set on one participant's entities must be readable by a remote participant.

use crate::{common::*, create_participant, delete_participant, wait_for};
use int2dds::{
    common::instance_handle::InstanceHandle,
    domain::{
        domain_participant_factory::DomainParticipantFactory,
        qos::{DomainParticipantFactoryQos, DomainParticipantQos},
    },
    infrastructure::{
        qos_policy::{EntityFactoryQosPolicy, UserDataQosPolicy},
        status::StatusMask,
    },
    publication::qos::{DataWriterQos, PublisherQos},
    subscription::qos::{DataReaderQos, SubscriberQos},
};

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
fn participant_user_data_change_is_visible_to_remote_participant() {
    let domain_id = next_domain_id();
    let initial_user_data = b"initial-user-data".to_vec();
    let changed_user_data = b"changed-user-data".to_vec();

    let sending_participant = create_participant(
        domain_id,
        DomainParticipantQos {
            user_data: UserDataQosPolicy { value: initial_user_data.clone() },
            ..Default::default()
        },
    );
    let receiving_participant = create_participant(domain_id, DomainParticipantQos::default());

    let sending_handle = sending_participant.get_instance_handle().unwrap();

    let is_initial_received = wait_for(|| {
        let participant_data =
            receiving_participant.get_discovered_participant_data(sending_handle).ok()?;
        (participant_data.user_data().value == initial_user_data).then_some(())
    });
    assert!(is_initial_received.is_some(), "initial user data was not received");

    let mut changed_qos = sending_participant.get_qos().unwrap();
    changed_qos.user_data = UserDataQosPolicy { value: changed_user_data.clone() };
    sending_participant.set_qos(changed_qos).unwrap();

    let is_changed_received = wait_for(|| {
        let participant_data =
            receiving_participant.get_discovered_participant_data(sending_handle).ok()?;
        (participant_data.user_data().value == changed_user_data).then_some(())
    });
    assert!(is_changed_received.is_some(), "changed user data was not received");

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

#[test]
fn participant_user_data_set_while_disabled_is_visible_after_enable() {
    let domain_id = next_domain_id();
    let initial_user_data = b"initial-user-data".to_vec();
    let changed_user_data = b"changed-user-data".to_vec();
    let factory = DomainParticipantFactory::get_instance();

    let original_factory_qos = factory.get_qos().unwrap();
    factory
        .set_qos(DomainParticipantFactoryQos {
            entity_factory: EntityFactoryQosPolicy { autoenable_created_entities: false },
        })
        .unwrap();
    let sending_participant = factory
        .create_participant(
            domain_id,
            DomainParticipantQos {
                user_data: UserDataQosPolicy { value: initial_user_data.clone() },
                ..Default::default()
            },
            None,
            StatusMask::default(),
        )
        .unwrap();
    factory.set_qos(original_factory_qos).unwrap();

    let receiving_participant = create_participant(domain_id, DomainParticipantQos::default());

    let mut changed_qos = sending_participant.get_qos().unwrap();
    changed_qos.user_data = UserDataQosPolicy { value: changed_user_data.clone() };
    sending_participant.set_qos(changed_qos).unwrap();

    sending_participant.enable().unwrap();

    let sending_handle = sending_participant.get_instance_handle().unwrap();

    let received_user_data = wait_for(|| {
        let participant_data =
            receiving_participant.get_discovered_participant_data(sending_handle).ok()?;
        Some(participant_data.user_data().value.clone())
    });
    assert_eq!(received_user_data, Some(changed_user_data));

    delete_participant(sending_participant);
    delete_participant(receiving_participant);
}
