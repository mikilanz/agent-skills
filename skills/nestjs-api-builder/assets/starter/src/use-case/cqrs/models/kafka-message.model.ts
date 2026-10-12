import { AggregateRoot } from '@nestjs/cqrs';
import { SendKafkaMessageEvent } from '../events/actions/send-kafka-message.event';

export class KafkaMessageModel extends AggregateRoot {
    constructor(private topic: string) {
        super();
        this.autoCommit = true;
    }

    sendMessage(message: object) {
        this.apply(new SendKafkaMessageEvent(this.topic, message));
    }
}
