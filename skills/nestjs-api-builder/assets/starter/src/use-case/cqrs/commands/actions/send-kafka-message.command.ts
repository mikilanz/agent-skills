export class SendKafkaMessageCommand {
    constructor(
        public readonly topic: string,
        public readonly message: object
    ) { }
}
