import { Module } from '@nestjs/common';
import { ThrottlerModule } from '@nestjs/throttler';
import { CqrsModule } from '@nestjs/cqrs';
import { ConfigModule } from './config.module';
import { TypeOrmModule } from './typeorm.module';
import { HttpModule } from './http.module';
import { CacheModule } from './cache.module';
import { KafkaModule } from './kafka.module';
import { HealthModule } from './health.module';

@Module({
    imports: [
        ConfigModule,
        TypeOrmModule,
        HttpModule,
        CacheModule,
        KafkaModule,
        ThrottlerModule.forRoot([{
            limit: 10,
            ttl: 60000
        }]),
        HealthModule,
        CqrsModule.forRoot()
    ]
})
export class GeneralModule { }
