import { Global, Module } from '@nestjs/common';
import { KafkaService } from 'src/use-case';

@Global()
@Module({
    providers: [KafkaService],
    exports: [KafkaService]
})
export class KafkaModule { }
