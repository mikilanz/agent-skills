export class SendKafkaMessageEvent {
    constructor(
        public readonly topic: string,
        public readonly message: object
    ) { }
}
