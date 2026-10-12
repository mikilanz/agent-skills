import { Injectable, Logger, OnModuleDestroy, OnModuleInit } from '@nestjs/common';
import { Kafka, Message, Producer } from 'kafkajs';
import { ConfigService } from 'src/config';

@Injectable()
export class KafkaService implements OnModuleInit, OnModuleDestroy {
    private readonly logger: Logger = new Logger(KafkaService.name);
    private readonly producer: Producer;

    constructor(private readonly configService: ConfigService) {
        this.producer = new Kafka(this.configService.getKafkaConfig()).producer();
    }

    async onModuleInit(): Promise<void> {
        try {
            await this.producer.connect();
        } catch (err) {
            this.logger.error(`Error connecting the producer: ${err}`);
        }
    }

    async onModuleDestroy(): Promise<void> {
        try {
            await this.producer.disconnect();
        } catch (err) {
            this.logger.error(`Error disconnecting the producer: ${err}`);
        }
    }

    async sendMessage(topic: string, message: string): Promise<void> {
        await this.producer.send({
            topic,
            messages: [{ value: message }],
            acks: 1
        });
    }

    async sendMessagesBatch(topic: string, messages: Array<object>): Promise<void> {
        const kafkaMessages: Array<Message> = messages.map((message) => ({ value: JSON.stringify(message) }));

        await this.producer.sendBatch({ topicMessages: [{ topic, messages: kafkaMessages }] });
    }
}
