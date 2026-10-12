import { MiddlewareConsumer, Module } from '@nestjs/common';
import { GeneralModule } from './application/modules';
import { ContextMiddleware } from './application/middleware/context.middleware';
import { KafkaMessageCommandHandler } from './use-case/cqrs/commands';
import { KafkaMessageEventHandler } from './use-case/cqrs/events';

@Module({
    imports: [
        GeneralModule,
        // feature modules
    ],
    providers: [
        ...KafkaMessageCommandHandler,
        ...KafkaMessageEventHandler
    ]
})
export class AppModule {
    configure(consumer: MiddlewareConsumer): any {
        consumer.apply(ContextMiddleware).forRoutes('*');
    }
}
