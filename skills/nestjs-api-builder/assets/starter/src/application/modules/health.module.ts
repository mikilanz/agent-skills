import { Module } from '@nestjs/common';
import { TerminusModule } from '@nestjs/terminus';
import { HealthController } from 'src/delivery/http';
import { RedisHealthIndicator } from '../indicators/redis-health.indicator';

@Module({
    imports: [
        TerminusModule.forRoot({
            gracefulShutdownTimeoutMs: 30000
        })
    ],
    controllers: [HealthController],
    providers: [RedisHealthIndicator],
    exports: [RedisHealthIndicator]
})
export class HealthModule { }
