import { SendKafkaMessageCommand } from './actions/send-kafka-message.command';
import { SendKafkaMessageCommandHandler } from './handlers/send-kafka-message.handler';

export const KafkaMessageCommandHandler = [
    SendKafkaMessageCommand,
    SendKafkaMessageCommandHandler
];
