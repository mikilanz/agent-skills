import { SendKafkaMessageEvent } from './actions/send-kafka-message.event';
import { SendKafkaMessageEventHandler } from './handlers/send-kafka-message.handler';

export const KafkaMessageEventHandler = [
    SendKafkaMessageEvent,
    SendKafkaMessageEventHandler
];
