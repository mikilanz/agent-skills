import { CommandHandler, EventPublisher, ICommandHandler } from '@nestjs/cqrs';
import { SendKafkaMessageCommand } from '../actions/send-kafka-message.command';
import { KafkaMessageModel } from '../../models';

@CommandHandler(SendKafkaMessageCommand)
export class SendKafkaMessageCommandHandler implements ICommandHandler<SendKafkaMessageCommand> {
    constructor(private publisher: EventPublisher) { }

    async execute(command: SendKafkaMessageCommand) {
        const { topic, message } = command;
        const kafkaMessage = this.publisher.mergeObjectContext(new KafkaMessageModel(topic));

        kafkaMessage.sendMessage(message);
    }
}
