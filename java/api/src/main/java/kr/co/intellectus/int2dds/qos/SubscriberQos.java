package kr.co.intellectus.int2dds.qos;

import java.util.Objects;

/**
 * Subscriber QoS.
 *
 * <p>Every field is nullable and defaults to null, meaning "leave it to the core".
 */
public class SubscriberQos {

    private Presentation presentation;
    private Partition partition;

    public SubscriberQos() {}

    public Presentation getPresentation() {
        return presentation;
    }

    public void setPresentation(Presentation presentation) {
        this.presentation = presentation;
    }

    public Partition getPartition() {
        return partition;
    }

    public void setPartition(Partition partition) {
        this.partition = partition;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) {
            return true;
        }
        if (!(o instanceof SubscriberQos)) {
            return false;
        }
        SubscriberQos other = (SubscriberQos) o;
        return Objects.equals(presentation, other.presentation)
                && Objects.equals(partition, other.partition);
    }

    @Override
    public int hashCode() {
        return Objects.hash(presentation, partition);
    }

    @Override
    public String toString() {
        return "SubscriberQos{presentation=" + presentation + ", partition=" + partition + "}";
    }
}
