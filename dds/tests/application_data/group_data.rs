// GROUP_DATA set on a Publisher or Subscriber must be readable by the matched remote endpoint.

use crate::{common::*, create_participant, delete_participant, wait_for};
use int2dds::{
    common::instance_handle::InstanceHandle,
    domain::qos::DomainParticipantQos,
    infrastructure::qos_policy::GroupDataQosPolicy,
    publication::{
        data_writer::DataWriter,
        qos::{DataWriterQos, PublisherQos},
    },
    subscription::{
        data_reader::DataReader,
        qos::{DataReaderQos, SubscriberQos},
    },
};

// Publisher GROUP_DATA of the matched writer as the reader sees it.
fn matched_publication_group_data(
    data_reader: &DataReader<KeyedDataType>,
    writer_handle: InstanceHandle,
) -> Option<Vec<u8>> {
    let publication_data = data_reader.get_matched_publication_data(writer_handle).ok()?;
    Some(publication_data.group_data().value.clone())
}

// Subscriber GROUP_DATA of the matched reader as the writer sees it.
fn matched_subscription_group_data(
    data_writer: &DataWriter<KeyedDataType>,
    reader_handle: InstanceHandle,
) -> Option<Vec<u8>> {
    let subscription_data = data_writer.get_matched_subscription_data(reader_handle).ok()?;
    Some(subscription_data.group_data().value.clone())
}

#[test]
fn group_data_is_visible_to_remote_endpoints() {
    let domain_id = next_domain_id();
    let publisher_group_data = b"publisher-group-data".to_vec();
    let subscriber_group_data = b"subscriber-group-data".to_vec();

    let writer_participant = create_participant(domain_id, DomainParticipantQos::default());
    let reader_participant = create_participant(domain_id, DomainParticipantQos::default());

    let data_writer = create_datawriter(
        &writer_participant,
        PublisherQos {
            group_data: GroupDataQosPolicy { value: publisher_group_data.clone() },
            ..Default::default()
        },
        DataWriterQos::default(),
    );
    let data_reader = create_datareader(
        &reader_participant,
        SubscriberQos {
            group_data: GroupDataQosPolicy { value: subscriber_group_data.clone() },
            ..Default::default()
        },
        DataReaderQos::default(),
    );

    let writer_handle = data_writer.get_instance_handle().unwrap();
    let reader_handle = data_reader.get_instance_handle().unwrap();

    let is_publisher_group_data_received = wait_for(|| {
        let group_data = matched_publication_group_data(&data_reader, writer_handle)?;
        (group_data == publisher_group_data).then_some(())
    });
    assert!(is_publisher_group_data_received.is_some(), "publisher group data was not received");

    let is_subscriber_group_data_received = wait_for(|| {
        let group_data = matched_subscription_group_data(&data_writer, reader_handle)?;
        (group_data == subscriber_group_data).then_some(())
    });
    assert!(is_subscriber_group_data_received.is_some(), "subscriber group data was not received");

    drop(data_writer);
    drop(data_reader);
    delete_participant(writer_participant);
    delete_participant(reader_participant);
}
