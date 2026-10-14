namespace Int2Dds.Qos
{
    public class PublisherQos
    {
        public Partition? Partition { get; set; }
        public Presentation? Presentation { get; set; }

        public PublisherQos() { }
    }
}
