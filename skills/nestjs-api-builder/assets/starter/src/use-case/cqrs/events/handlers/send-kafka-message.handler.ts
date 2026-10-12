import { EventsHandler, IEventHandler } from '@nestjs/cqrs';
import { Logger } from '@nestjs/common';
import { catchError, from, of } from 'rxjs';
import { KafkaService } from 'src/use-case/kafka/kafka.service';
import { SendKafkaMessageEvent } from '../actions/send-kafka-message.event';

@EventsHandler(SendKafkaMessageEvent)
export class SendKafkaMessageEventHandler implements IEventHandler<SendKafkaMessageEvent> {
    private readonly logger: Logger = new Logger(SendKafkaMessageEventHandler.name);

    constructor(private readonly kafkaService: KafkaService) { }

    handle(event: SendKafkaMessageEvent) {
        const { topic, message } = event;

        from(this.kafkaService.sendMessage(topic, JSON.stringify(message))).pipe(
            catchError((err) => {
                this.logger.error(`Error sending Kafka message: ${err}`);
                return of(null);
            })
        ).subscribe(() => this.logger.log(`Message sent to topic ${topic}`));
    }
}
