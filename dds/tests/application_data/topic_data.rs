// TOPIC_DATA set on a Topic must be readable by the matched remote endpoint.

use crate::{common::*, create_participant, delete_participant, wait_for};
use int2dds::{
    domain::{domain_participant::DomainParticipant, qos::DomainParticipantQos},
    infrastructure::{qos_policy::TopicDataQosPolicy, status::StatusMask},
    publication::qos::{DataWriterQos, PublisherQos},
    subscription::qos::{DataReaderQos, SubscriberQos},
    topic::{qos::TopicQos, topic::Topic},
};

fn create_topic(participant: &DomainParticipant, topic_data: &[u8]) -> Topic {
    participant
        .create_topic::<KeyedDataType>(
            KeyedDataType::get_topic_name(),
            KeyedDataType::get_type_name(),
            TopicQos {
                topic_data: TopicDataQosPolicy { value: topic_data.to_vec() },
                ..Default::default()
            },
            None,
            StatusMask::default(),
        )
        .unwrap()
}

#[test]
fn topic_data_is_visible_to_remote_endpoints() {
    let domain_id = next_domain_id();
    let writer_topic_data = b"writer-topic-data".to_vec();
    let reader_topic_data = b"reader-topic-data".to_vec();

    let writer_participant = create_participant(domain_id, DomainParticipantQos::default());
    let reader_participant = create_participant(domain_id, DomainParticipantQos::default());

    let writer_topic = create_topic(&writer_participant, &writer_topic_data);
    let publisher = writer_participant
        .create_publisher(PublisherQos::default(), None, StatusMask::default())
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<KeyedDataType>(
            &writer_topic,
            DataWriterQos::default(),
            None,
            StatusMask::default(),
        )
        .unwrap();

    let reader_topic = create_topic(&reader_participant, &reader_topic_data);
    let subscriber = reader_participant
        .create_subscriber(SubscriberQos::default(), None, StatusMask::default())
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<KeyedDataType>(
            &reader_topic,
            DataReaderQos::default(),
            None,
            StatusMask::default(),
        )
        .unwrap();

    let writer_handle = data_writer.get_instance_handle().unwrap();
    let reader_handle = data_reader.get_instance_handle().unwrap();

    let is_writer_topic_data_received = wait_for(|| {
        let publication_data = data_reader.get_matched_publication_data(writer_handle).ok()?;
        (publication_data.topic_data().value == writer_topic_data).then_some(())
    });
    assert!(is_writer_topic_data_received.is_some(), "writer topic data was not received");

    let is_reader_topic_data_received = wait_for(|| {
        let subscription_data = data_writer.get_matched_subscription_data(reader_handle).ok()?;
        (subscription_data.topic_data().value == reader_topic_data).then_some(())
    });
    assert!(is_reader_topic_data_received.is_some(), "reader topic data was not received");

    drop(data_writer);
    drop(data_reader);
    delete_participant(writer_participant);
    delete_participant(reader_participant);
}
